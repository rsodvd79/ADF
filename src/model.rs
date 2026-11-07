use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const MAX_CONTROLS: usize = 1_000;
pub const MAX_DEPTH: usize = 32;
pub const MAX_TEXT: usize = 1_048_576;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename = "ADF", deny_unknown_fields)]
pub struct Document {
    #[serde(rename = "@VERSION", default = "legacy_version")]
    pub version: u32,
    #[serde(rename = "@SCRIPT_LANGUAGE", default = "legacy_language")]
    pub language: String,
    #[serde(
        rename = "@xmlns:xsi",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub xmlns_xsi: Option<String>,
    #[serde(
        rename = "@xmlns:xsd",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub xmlns_xsd: Option<String>,
    #[serde(rename = "PROPERTY")]
    pub property: ProjectProperty,
    #[serde(rename = "MODULO", default)]
    pub module: String,
    #[serde(rename = "CONTROLS", default)]
    pub controls: Controls,
}

fn legacy_version() -> u32 {
    1
}

fn legacy_language() -> String {
    "vbscript".into()
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectProperty {
    #[serde(rename = "STARTCONTROL", default)]
    pub start: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Controls {
    #[serde(rename = "CONTROL", default)]
    pub items: Vec<Control>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Control {
    #[serde(rename = "PROPERTY")]
    pub property: Property,
    #[serde(rename = "EVENTS", default)]
    pub events: Events,
    #[serde(rename = "CONTROLS", default)]
    pub controls: Controls,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Kind {
    #[default]
    Form,
    Button,
    Label,
    Textbox,
    Checkbox,
    Combobox,
}

impl Kind {
    pub const ALL: [Self; 6] = [
        Self::Form,
        Self::Button,
        Self::Label,
        Self::Textbox,
        Self::Checkbox,
        Self::Combobox,
    ];
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Layout {
    #[default]
    Absolute,
    Vertical,
    Horizontal,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Property {
    #[serde(rename = "TYPE", default)]
    pub kind: Kind,
    #[serde(rename = "NAME", default)]
    pub name: String,
    #[serde(rename = "TEXT", default)]
    pub text: String,
    #[serde(rename = "WIDTH", default = "default_width")]
    pub width: f32,
    #[serde(rename = "HEIGHT", default = "default_height")]
    pub height: f32,
    #[serde(rename = "TOP", default)]
    pub top: f32,
    #[serde(rename = "LEFT", default)]
    pub left: f32,
    #[serde(rename = "LAYOUT", default)]
    pub layout: Layout,
    #[serde(rename = "SPACING", default = "default_spacing")]
    pub spacing: f32,
    #[serde(rename = "PADDING", default = "default_spacing")]
    pub padding: f32,
    #[serde(rename = "EXPAND", default)]
    pub expand: bool,
    #[serde(rename = "CHECKED", default)]
    pub checked: bool,
    #[serde(rename = "SELECTED", default)]
    pub selected: usize,
    #[serde(rename = "OPTIONS", default)]
    pub options: Options,
}

fn default_width() -> f32 {
    120.0
}
fn default_height() -> f32 {
    32.0
}
fn default_spacing() -> f32 {
    8.0
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Options {
    #[serde(rename = "OPTION", default)]
    pub items: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Events {
    #[serde(rename = "CLICK", default)]
    pub click: String,
    #[serde(rename = "CHANGE", default)]
    pub change: String,
}

impl Control {
    pub fn new(kind: Kind, name: String) -> Self {
        let form = kind == Kind::Form;
        Self {
            property: Property {
                kind,
                text: name.clone(),
                name,
                width: if form { 640.0 } else { 120.0 },
                height: if form { 480.0 } else { 32.0 },
                top: 8.0,
                left: 8.0,
                layout: Layout::default(),
                spacing: 8.0,
                padding: 8.0,
                expand: false,
                checked: false,
                selected: 0,
                options: Options {
                    items: if kind == Kind::Combobox {
                        vec!["First".into(), "Second".into()]
                    } else {
                        vec![]
                    },
                },
            },
            events: Events::default(),
            controls: Controls::default(),
        }
    }
}

impl Default for Document {
    fn default() -> Self {
        Self {
            version: 2,
            language: "rhai".into(),
            xmlns_xsi: None,
            xmlns_xsd: None,
            property: ProjectProperty {
                start: "MAIN".into(),
            },
            module: String::new(),
            controls: Controls {
                items: vec![Control::new(Kind::Form, "MAIN".into())],
            },
        }
    }
}

impl Document {
    pub fn is_legacy(&self) -> bool {
        self.version == 1 && self.language == "vbscript"
    }

    pub fn validate(&self) -> Result<(), String> {
        if !self.is_legacy() && (self.version != 2 || self.language != "rhai") {
            return Err(format!(
                "Unsupported VERSION={} SCRIPT_LANGUAGE={}",
                self.version, self.language
            ));
        }
        if self.module.len() > MAX_TEXT {
            return Err("MODULO exceeds 1 MiB".into());
        }
        let mut names = HashSet::new();
        let mut legacy_names = HashSet::new();
        let mut count = 0;
        for form in &self.controls.items {
            if form.property.kind != Kind::Form {
                return Err("Only FORM controls can appear at the project root".into());
            }
            Self::validate_control(
                form,
                0,
                &mut names,
                &mut legacy_names,
                &mut count,
                self.is_legacy(),
            )?;
        }
        if !self
            .controls
            .items
            .iter()
            .any(|c| c.property.name == self.property.start)
        {
            return Err(format!(
                "STARTCONTROL '{}' does not reference a FORM",
                self.property.start
            ));
        }
        Ok(())
    }

    fn validate_control(
        c: &Control,
        depth: usize,
        names: &mut HashSet<String>,
        legacy_names: &mut HashSet<String>,
        count: &mut usize,
        legacy: bool,
    ) -> Result<(), String> {
        *count += 1;
        if *count > MAX_CONTROLS || depth > MAX_DEPTH {
            return Err("Control count or nesting limit exceeded".into());
        }
        let p = &c.property;
        let fail = |message| Err(format!("CONTROL '{}': {message}", p.name));
        if p.name.is_empty() || p.name.len() > 128 || p.name.chars().any(char::is_control) {
            return fail("NAME must contain 1..128 bytes without control characters");
        }
        if !names.insert(p.name.clone()) || (legacy && !legacy_names.insert(p.name.to_lowercase()))
        {
            return fail("duplicate NAME (legacy names are case-insensitive)");
        }
        for value in [p.width, p.height, p.left, p.top, p.spacing, p.padding] {
            if !value.is_finite() || !(0.0..=16_384.0).contains(&value) {
                return fail("geometry must be finite and in 0..16384");
            }
        }
        if p.width < 1.0 || p.height < 1.0 {
            return fail("WIDTH and HEIGHT must be positive");
        }
        if depth > 0 && p.kind == Kind::Form {
            return fail("nested FORM is unsupported; move it to the project root");
        }
        if p.kind != Kind::Form && !c.controls.items.is_empty() {
            return fail("only FORM may contain child controls");
        }
        if !c.events.click.trim().is_empty() && p.kind != Kind::Button {
            return fail("CLICK is supported only on BUTTON");
        }
        if !c.events.change.trim().is_empty()
            && !matches!(p.kind, Kind::Textbox | Kind::Checkbox | Kind::Combobox)
        {
            return fail("CHANGE requires TEXTBOX, CHECKBOX or COMBOBOX");
        }
        if [p.text.len(), c.events.click.len(), c.events.change.len()]
            .into_iter()
            .any(|n| n > MAX_TEXT)
        {
            return fail("text or script exceeds 1 MiB");
        }
        if p.options.items.len() > 10_000 || p.options.items.iter().any(|s| s.len() > MAX_TEXT) {
            return fail("OPTIONS exceed limits");
        }
        if p.kind == Kind::Combobox
            && (p.options.items.is_empty() || p.selected >= p.options.items.len())
        {
            return fail("COMBOBOX requires options and a valid zero-based SELECTED index");
        }
        for child in &c.controls.items {
            Self::validate_control(child, depth + 1, names, legacy_names, count, legacy)?;
        }
        Ok(())
    }

    pub fn find(&self, name: &str) -> Option<&Control> {
        self.controls.items.iter().find_map(|form| {
            if form.property.name == name {
                Some(form)
            } else {
                form.controls.items.iter().find(|c| c.property.name == name)
            }
        })
    }

    pub fn find_mut(&mut self, name: &str) -> Option<&mut Control> {
        self.controls.items.iter_mut().find_map(|form| {
            if form.property.name == name {
                Some(form)
            } else {
                form.controls
                    .items
                    .iter_mut()
                    .find(|c| c.property.name == name)
            }
        })
    }

    pub fn unique_name(&self, base: &str) -> String {
        (1..)
            .map(|n| format!("{base}_{n}"))
            .find(|name| self.find(name).is_none())
            .unwrap_or_else(|| base.into())
    }
}
