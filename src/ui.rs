use crate::{
    document::{self, OpenFile},
    editor::Editor,
    model::{Control, Kind, Layout, Property},
    runtime::{Dialog, Session, Snapshot, Update},
};
use eframe::egui::{self, Color32, Id, Pos2, Rect, Response, Sense, Vec2};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, path::PathBuf, time::Duration};

#[derive(Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct Preferences {
    theme: Theme,
    recent: Vec<PathBuf>,
}

#[derive(Clone)]
enum Action {
    New,
    Open(PathBuf),
    Reload,
    Close,
}

#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Design,
    Xml,
    Preview,
}

struct Drag {
    name: String,
    original: Property,
    resize: bool,
    pointer: Pos2,
}

pub struct AdfApp {
    editor: Editor,
    file: Option<OpenFile>,
    preferences: Preferences,
    tab: Tab,
    session: Option<Session>,
    retired: Vec<Session>,
    snapshot: Option<Snapshot>,
    dialog: Option<Dialog>,
    errors: Vec<String>,
    pending: Option<Action>,
    closing: bool,
    rename_for: String,
    rename: String,
    drag: Option<Drag>,
}

#[cfg(feature = "native-smoke")]
#[path = "native_smoke.rs"]
mod native_smoke;

#[cfg(feature = "native-smoke")]
pub use native_smoke::run as run_native_smoke;

impl AdfApp {
    pub fn new(cc: &eframe::CreationContext<'_>, path: Option<PathBuf>) -> Self {
        let preferences = cc
            .storage
            .and_then(|s| eframe::get_value(s, "preferences"))
            .unwrap_or_default();
        let mut app = Self {
            editor: Editor::default(),
            file: None,
            preferences,
            tab: Tab::Design,
            session: None,
            retired: vec![],
            snapshot: None,
            dialog: None,
            errors: vec![],
            pending: None,
            closing: false,
            rename_for: String::new(),
            rename: String::new(),
            drag: None,
        };
        app.set_theme(&cc.egui_ctx);
        if let Some(path) = path {
            app.open(path);
        }
        app
    }

    fn report(&mut self, result: Result<(), String>) {
        if let Err(e) = result {
            self.editor.error = Some(e.clone());
            self.log(e);
        }
    }

    fn log(&mut self, text: String) {
        if self.errors.len() == 100 {
            self.errors.remove(0);
        }
        self.errors.push(text);
    }

    fn set_theme(&self, ctx: &egui::Context) {
        ctx.set_theme(match self.preferences.theme {
            Theme::System => egui::ThemePreference::System,
            Theme::Light => egui::ThemePreference::Light,
            Theme::Dark => egui::ThemePreference::Dark,
        });
    }

    fn remember(&mut self, path: PathBuf) {
        self.preferences.recent.retain(|p| p != &path);
        self.preferences.recent.insert(0, path);
        self.preferences.recent.truncate(10);
    }

    fn open(&mut self, path: PathBuf) {
        match document::load(&path)
            .and_then(|source| document::parse(&source).map(|doc| (source, doc)))
        {
            Ok((source, doc)) => {
                self.stop();
                self.editor = Editor::from_doc(doc);
                self.editor.source = source.clone();
                self.editor.applied_source = source.clone();
                self.editor.saved_source = source.clone();
                self.file = Some(OpenFile {
                    path: path.clone(),
                    disk_source: source,
                });
                self.remember(path);
                self.rename_for.clear();
            }
            Err(e) => self.report(Err(e)),
        }
    }

    fn save_file(&mut self, save_as: bool) -> bool {
        let path = if save_as || self.file.is_none() {
            rfd::FileDialog::new()
                .add_filter("ADF XML", &["xml"])
                .set_file_name("project.xml")
                .save_file()
        } else {
            self.file.as_ref().map(|f| f.path.clone())
        };
        let Some(path) = path else {
            return false;
        };
        if self
            .file
            .as_ref()
            .is_some_and(|f| f.path == path && f.changed_on_disk())
        {
            let answer = rfd::MessageDialog::new()
                .set_title("File changed on disk")
                .set_description("Overwrite the external changes? Reload instead to keep them.")
                .set_buttons(rfd::MessageButtons::YesNo)
                .show();
            if answer != rfd::MessageDialogResult::Yes {
                return false;
            }
        }
        if let Err(e) = self
            .editor
            .apply()
            .and_then(|()| document::save(&path, &self.editor.source))
        {
            self.report(Err(e));
            return false;
        }
        self.editor.saved_source = self.editor.source.clone();
        self.file = Some(OpenFile {
            path: path.clone(),
            disk_source: self.editor.source.clone(),
        });
        self.remember(path);
        true
    }

    fn request_action(&mut self, action: Action, ctx: &egui::Context) {
        if self.editor.dirty() {
            self.pending = Some(action);
        } else {
            self.execute_action(action, ctx);
        }
    }

    fn execute_action(&mut self, action: Action, ctx: &egui::Context) {
        self.drag = None;
        match action {
            Action::New => {
                self.stop();
                self.editor = Editor::default();
                self.file = None;
                self.rename_for.clear();
            }
            Action::Open(path) => self.open(path),
            Action::Reload => {
                self.stop();
                if let Some(file) = &self.file {
                    self.open(file.path.clone());
                }
            }
            Action::Close => {
                self.stop();
                self.closing = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }

    fn stop(&mut self) {
        if let Some(mut session) = self.session.take() {
            session.stop();
            self.retired.push(session);
        }
        self.dialog = None;
        self.snapshot = None;
    }

    fn run(&mut self, ctx: &egui::Context) {
        if self.editor.pending()
            && let Err(e) = self.editor.apply()
        {
            self.report(Err(e));
            return;
        }
        self.stop();
        let wake = ctx.clone();
        match Session::start(self.editor.doc.clone(), move || wake.request_repaint()) {
            Ok(session) => {
                self.snapshot = Some(Snapshot {
                    doc: self.editor.doc.clone(),
                    visible: HashSet::from([self.editor.doc.property.start.clone()]),
                });
                self.session = Some(session);
            }
            Err(e) => self.report(Err(e)),
        }
    }

    fn poll(&mut self, ctx: &egui::Context) {
        let updates: Vec<_> = self
            .session
            .as_ref()
            .map(|s| s.updates.try_iter().collect())
            .unwrap_or_default();
        let mut finished = false;
        for update in updates {
            match update {
                Update::State(state) => self.snapshot = Some(*state),
                Update::Dialog(dialog) => {
                    if self
                        .session
                        .as_ref()
                        .is_some_and(|s| s.id == dialog.session)
                    {
                        self.dialog = Some(dialog);
                    }
                }
                Update::Ready => {
                    if let Some(s) = &mut self.session {
                        s.busy = false;
                    }
                }
                Update::Error(e) => self.report(Err(e)),
                Update::Log(text) => {
                    self.log(format!("print: {text}"));
                }
                Update::Finished => finished = true,
            }
        }
        if finished
            || self
                .snapshot
                .as_ref()
                .is_some_and(|s| !s.visible.contains(&s.doc.property.start))
        {
            self.stop();
        }
        self.retired.retain_mut(|s| {
            if s.is_finished() {
                s.join();
                false
            } else {
                true
            }
        });
        if self.session.as_ref().is_some_and(|s| s.busy) || !self.retired.is_empty() {
            ctx.request_repaint_after(Duration::from_millis(10));
        }
    }

    fn toolbar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                if ui.button("New").clicked() {
                    self.request_action(Action::New, ctx);
                }
                for label in ["Open", "Import legacy"] {
                    if ui.button(label).clicked()
                        && let Some(path) = rfd::FileDialog::new()
                            .add_filter("ADF XML", &["xml", "XML"])
                            .pick_file()
                    {
                        self.request_action(Action::Open(path), ctx);
                    }
                }
                if ui.button("Save").clicked() {
                    self.save_file(false);
                }
                if ui.button("Save as").clicked() {
                    self.save_file(true);
                }
                if ui
                    .add_enabled(self.file.is_some(), egui::Button::new("Reload"))
                    .clicked()
                {
                    self.request_action(Action::Reload, ctx);
                }
                ui.separator();
                if ui.button("Run").clicked() {
                    self.run(ctx);
                }
                if ui
                    .add_enabled(self.session.is_some(), egui::Button::new("Stop"))
                    .clicked()
                {
                    self.stop();
                }
                ui.menu_button("Recent", |ui| {
                    for path in self.preferences.recent.clone() {
                        if ui.button(path.display().to_string()).clicked() {
                            self.request_action(Action::Open(path), ctx);
                            ui.close();
                        }
                    }
                });
                let before = self.preferences.theme;
                egui::ComboBox::from_id_salt("theme")
                    .selected_text(match before {
                        Theme::System => "System",
                        Theme::Light => "Light",
                        Theme::Dark => "Dark",
                    })
                    .show_ui(ui, |ui| {
                        for (theme, label) in [
                            (Theme::System, "System"),
                            (Theme::Light, "Light"),
                            (Theme::Dark, "Dark"),
                        ] {
                            ui.selectable_value(&mut self.preferences.theme, theme, label);
                        }
                    });
                if before != self.preferences.theme {
                    self.set_theme(ctx);
                }
            });
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.tab, Tab::Design, "Designer");
                ui.selectable_value(&mut self.tab, Tab::Xml, "XML");
                ui.selectable_value(&mut self.tab, Tab::Preview, "Preview (no scripts)");
                ui.separator();
                ui.label(format!(
                    "{}{}",
                    self.file
                        .as_ref()
                        .map_or("Untitled".into(), |f| f.path.display().to_string()),
                    if self.editor.dirty() { " *" } else { "" }
                ));
                if self.session.is_some() {
                    ui.label("Running snapshot: edits apply on next Run");
                }
            });
        });
        egui::TopBottomPanel::bottom("diagnostics")
            .resizable(true)
            .default_height(64.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Diagnostics");
                    if ui.small_button("Clear").clicked() {
                        self.errors.clear();
                        self.editor.error = None;
                    }
                });
                egui::ScrollArea::vertical()
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        for error in &self.errors {
                            ui.colored_label(Color32::from_rgb(225, 100, 100), error);
                        }
                    });
            });
    }

    fn tree(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("controls").resizable(true).default_width(190.0).show(ctx, |ui| {
            ui.heading("Controls");
            if self.editor.doc.is_legacy() {
                ui.colored_label(Color32::YELLOW, "VBScript import: not executable");
                if ui.button("Mark scripts converted").clicked() {
                    let answer = rfd::MessageDialog::new().set_title("Confirm script migration")
                        .set_description("Have you converted MODULO and every event to Rhai? This only changes VERSION/language; it does not translate code.")
                        .set_buttons(rfd::MessageButtons::YesNo).show();
                    if answer == rfd::MessageDialogResult::Yes {
                        let result = self.editor.edit(|d| { d.version = 2; d.language = "rhai".into(); });
                        self.report(result);
                    }
                }
            }
            egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                for form in &self.editor.doc.controls.items {
                    if ui.selectable_label(self.editor.selected == form.property.name, format!("FORM {}", form.property.name)).clicked() {
                        self.editor.selected = form.property.name.clone();
                    }
                    ui.indent(&form.property.name, |ui| {
                        for c in &form.controls.items {
                            if ui.selectable_label(self.editor.selected == c.property.name,
                                format!("{:?} {}", c.property.kind, c.property.name)).clicked()
                            { self.editor.selected = c.property.name.clone(); }
                        }
                    });
                }
            });
            ui.separator();
            ui.label("Palette: click to add or drag onto canvas");
            ui.add_enabled_ui(!self.editor.pending(), |ui| {
                for kind in Kind::ALL {
                    let clicked = ui.dnd_drag_source(Id::new(("palette", format!("{kind:?}"))), kind, |ui| {
                        ui.button(format!("Add {kind:?}")).clicked()
                    }).inner;
                    if clicked {
                        let result = self.editor.add(kind, None);
                        self.report(result);
                    }
                }
                if ui.button("Duplicate selected").clicked() {
                    let result = self.editor.duplicate_selected();
                    let success = result.is_ok();
                    self.report(result);
                    if success {
                        self.log("Duplicated scripts retain their original control references; review them.".into());
                    }
                }
                if ui.button("Delete selected").clicked() {
                    let result = self.editor.delete_selected();
                    self.report(result);
                }
                ui.horizontal(|ui| {
                    if ui.button("Undo").clicked() { self.editor.undo(); }
                    if ui.button("Redo").clicked() { self.editor.redo(); }
                });
            });
        });
    }

    fn inspector(&mut self, ctx: &egui::Context) {
        egui::SidePanel::right("properties")
            .resizable(true)
            .default_width(270.0)
            .show(ctx, |ui| {
                ui.heading("Properties / events");
                ui.add_enabled_ui(!self.editor.pending(), |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        let selected = self.editor.selected.clone();
                        if let Some(mut c) = self.editor.doc.find(&selected).cloned() {
                            let before = c.clone();
                            if self.rename_for != selected {
                                self.rename_for = selected.clone();
                                self.rename = selected.clone();
                            }
                            ui.label(format!("{:?}", c.property.kind));
                            ui.text_edit_singleline(&mut self.rename);
                            if ui.button("Rename").clicked() {
                                let result = self.editor.rename_selected(self.rename.clone());
                                let success = result.is_ok();
                                self.report(result);
                                if success {
                                    self.log(
                                        "NAME changed: review string references in Rhai scripts."
                                            .into(),
                                    );
                                    return;
                                }
                            }
                            ui.label("Text");
                            ui.text_edit_singleline(&mut c.property.text);
                            egui::Grid::new("geometry").num_columns(2).show(ui, |ui| {
                                for (label, value, min) in [
                                    ("Width", &mut c.property.width, 1.0),
                                    ("Height", &mut c.property.height, 1.0),
                                    ("Left", &mut c.property.left, 0.0),
                                    ("Top", &mut c.property.top, 0.0),
                                    ("Padding", &mut c.property.padding, 0.0),
                                    ("Spacing", &mut c.property.spacing, 0.0),
                                ] {
                                    ui.label(label);
                                    ui.add(egui::DragValue::new(value).range(min..=16_384.0));
                                    ui.end_row();
                                }
                            });
                            if c.property.kind == Kind::Form {
                                egui::ComboBox::from_id_salt("layout")
                                    .selected_text(format!("{:?}", c.property.layout))
                                    .show_ui(ui, |ui| {
                                        for layout in
                                            [Layout::Absolute, Layout::Vertical, Layout::Horizontal]
                                        {
                                            ui.selectable_value(
                                                &mut c.property.layout,
                                                layout,
                                                format!("{layout:?}"),
                                            );
                                        }
                                    });
                                if ui.button("Use as STARTCONTROL").clicked() {
                                    let result =
                                        self.editor.edit(|d| d.property.start = selected.clone());
                                    self.report(result);
                                }
                            } else {
                                ui.checkbox(&mut c.property.expand, "Expand in adaptive layout");
                            }
                            if c.property.kind == Kind::Checkbox {
                                ui.checkbox(&mut c.property.checked, "Initial checked");
                            }
                            if c.property.kind == Kind::Combobox {
                                ui.label("Options");
                                for option in &mut c.property.options.items {
                                    ui.text_edit_singleline(option);
                                }
                                if ui.button("Add option").clicked() {
                                    c.property.options.items.push("Option".into());
                                }
                                if ui
                                    .add_enabled(
                                        c.property.options.items.len() > 1,
                                        egui::Button::new("Remove last option"),
                                    )
                                    .clicked()
                                {
                                    c.property.options.items.pop();
                                    c.property.selected =
                                        c.property.selected.min(c.property.options.items.len() - 1);
                                }
                                ui.horizontal(|ui| {
                                    ui.label("Selected (zero-based)");
                                    ui.add(egui::DragValue::new(&mut c.property.selected).range(
                                        0..=c.property.options.items.len().saturating_sub(1),
                                    ));
                                });
                            }
                            if c.property.kind == Kind::Button {
                                ui.label("CLICK (Rhai body)");
                                ui.add(
                                    egui::TextEdit::multiline(&mut c.events.click)
                                        .code_editor()
                                        .desired_rows(8),
                                );
                            }
                            if matches!(
                                c.property.kind,
                                Kind::Textbox | Kind::Checkbox | Kind::Combobox
                            ) {
                                ui.label("CHANGE (Rhai body)");
                                ui.add(
                                    egui::TextEdit::multiline(&mut c.events.change)
                                        .code_editor()
                                        .desired_rows(8),
                                );
                            }
                            if c != before {
                                let result = self.editor.edit(|d| {
                                    if let Some(target) = d.find_mut(&selected) {
                                        *target = c;
                                    }
                                });
                                self.report(result);
                            }
                        }
                        ui.separator();
                        ui.collapsing("MODULO (project initialization and functions)", |ui| {
                            let mut module = self.editor.doc.module.clone();
                            if ui
                                .add(
                                    egui::TextEdit::multiline(&mut module)
                                        .code_editor()
                                        .desired_rows(10),
                                )
                                .changed()
                            {
                                let result = self.editor.edit(|d| d.module = module);
                                self.report(result);
                            }
                        });
                    });
                });
            });
    }

    fn designer(&mut self, ui: &mut egui::Ui) {
        if self.editor.pending() {
            ui.colored_label(
                Color32::YELLOW,
                "Apply or discard pending XML changes to unlock the designer.",
            );
        }
        let Some(form) = self.editor.selected_form().cloned() else {
            return;
        };
        ui.label(format!(
            "{} - {:?} layout",
            form.property.name, form.property.layout
        ));
        ui.label("Absolute: drag to move, bottom-right handle to resize. Adaptive: drag onto a sibling to reorder.");
        egui::ScrollArea::both().show(ui, |ui| {
            let (rect, canvas) = ui.allocate_exact_size(
                Vec2::new(form.property.width, form.property.height),
                Sense::hover(),
            );
            ui.painter()
                .rect_filled(rect, 0.0, ui.visuals().extreme_bg_color);
            let mut preview = form.clone();
            ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                draw_form(ui, &mut preview, false, rect.size());
            });
            if self.editor.pending() {
                return;
            }
            if let Some(kind) = canvas.dnd_release_payload::<Kind>() {
                self.editor.selected = form.property.name.clone();
                let position = ui.ctx().pointer_latest_pos().map(|pointer| {
                    let pos = pointer - rect.min;
                    (pos.x, pos.y)
                });
                let result = self.editor.add(*kind, position);
                self.report(result);
            }
            let positions = placements(&form, rect.size());
            for (index, c) in form.controls.items.iter().enumerate() {
                let item_rect = positions[index].translate(rect.min.to_vec2());
                let response = ui.interact(
                    item_rect,
                    Id::new(("design", &c.property.name)),
                    Sense::click_and_drag(),
                );
                if response.clicked() {
                    self.editor.selected = c.property.name.clone();
                }
                let selected = self.editor.selected == c.property.name;
                if selected {
                    ui.painter().rect_stroke(
                        item_rect,
                        0.0,
                        (2.0, Color32::LIGHT_BLUE),
                        egui::StrokeKind::Inside,
                    );
                }
                if form.property.layout == Layout::Absolute {
                    let handle =
                        Rect::from_min_size(item_rect.max - Vec2::splat(10.0), Vec2::splat(10.0));
                    let resize =
                        ui.interact(handle, Id::new(("resize", &c.property.name)), Sense::drag());
                    if selected {
                        ui.painter().rect_filled(handle, 0.0, Color32::LIGHT_BLUE);
                    }
                    let active =
                        if resize.dragged() || resize.drag_started() || resize.drag_stopped() {
                            &resize
                        } else {
                            &response
                        };
                    if active.drag_started() {
                        self.editor.selected = c.property.name.clone();
                        self.editor.begin_gesture();
                        self.drag = Some(Drag {
                            name: c.property.name.clone(),
                            original: c.property.clone(),
                            resize: resize.drag_started(),
                            pointer: ui
                                .ctx()
                                .input(|i| i.pointer.press_origin())
                                .unwrap_or(item_rect.min),
                        });
                    }
                    if active.dragged()
                        && let Some(drag) = &self.drag
                    {
                        let delta =
                            ui.ctx().pointer_latest_pos().unwrap_or(drag.pointer) - drag.pointer;
                        let name = drag.name.clone();
                        let original = drag.original.clone();
                        let resize = drag.resize;
                        let result = self.editor.update_gesture(|d| {
                            if let Some(c) = d.find_mut(&name) {
                                if resize {
                                    c.property.width =
                                        (original.width + delta.x).clamp(1.0, 16_384.0);
                                    c.property.height =
                                        (original.height + delta.y).clamp(1.0, 16_384.0);
                                } else {
                                    c.property.left =
                                        (original.left + delta.x).clamp(0.0, 16_384.0);
                                    c.property.top = (original.top + delta.y).clamp(0.0, 16_384.0);
                                }
                            }
                        });
                        self.report(result);
                    }
                    if active.drag_stopped() {
                        self.editor.end_gesture();
                        self.drag = None;
                    }
                } else {
                    if response.drag_started() {
                        self.drag = Some(Drag {
                            name: c.property.name.clone(),
                            original: c.property.clone(),
                            resize: false,
                            pointer: item_rect.min,
                        });
                    }
                    if response.drag_stopped()
                        && let Some(drag) = self.drag.take()
                        && let Some(pointer) = ui.ctx().pointer_latest_pos()
                        && let Some(target) = positions
                            .iter()
                            .position(|r| r.translate(rect.min.to_vec2()).contains(pointer))
                    {
                        let form_name = form.property.name.clone();
                        let result = self.editor.edit(|d| {
                            if let Some(f) = d.find_mut(&form_name)
                                && let Some(from) = f
                                    .controls
                                    .items
                                    .iter()
                                    .position(|c| c.property.name == drag.name)
                            {
                                let c = f.controls.items.remove(from);
                                f.controls.items.insert(target, c);
                            }
                        });
                        self.report(result);
                    }
                }
            }
            if ui.ctx().input(|i| i.pointer.any_released()) && self.drag.is_some() {
                self.editor.end_gesture();
                self.drag = None;
            }
        });
    }

    fn runtime_windows(&mut self, ctx: &egui::Context) {
        let Some(mut snapshot) = self.snapshot.take() else {
            return;
        };
        let mut stop = false;
        for form in &mut snapshot.doc.controls.items {
            let name = form.property.name.clone();
            if !snapshot.visible.contains(&name) {
                continue;
            }
            let busy = self.session.as_ref().is_none_or(|s| s.busy) || self.dialog.is_some();
            let mut close = false;
            let mut events = vec![];
            let id = Id::new(("runtime", self.session.as_ref().map(|s| s.id), &name));
            ctx.show_viewport_immediate(
                egui::ViewportId(id),
                egui::ViewportBuilder::default()
                    .with_title(&form.property.text)
                    .with_inner_size(Vec2::new(form.property.width, form.property.height)),
                |ctx, _| {
                    close = ctx.input(|i| i.viewport().close_requested());
                    egui::CentralPanel::default()
                        .frame(egui::Frame::default())
                        .show(ctx, |ui| {
                            let size = ui.available_size();
                            egui::ScrollArea::both().show(ui, |ui| {
                                events = draw_form(ui, form, !busy, size);
                            });
                        });
                },
            );
            if close {
                if name == snapshot.doc.property.start || busy {
                    stop = true;
                } else if let Some(s) = &mut self.session {
                    let result = s.hide_form(&name);
                    self.report(result);
                    snapshot.visible.remove(&name);
                }
            }
            for event in events {
                if let Some(s) = &mut self.session {
                    let result = s.event(&event.name, event.change, event.property);
                    self.report(result);
                }
            }
        }
        self.snapshot = Some(snapshot);
        if stop {
            self.stop();
        }
        if let Some(dialog) = &self.dialog {
            let mut choice = None;
            let mut closed = false;
            ctx.show_viewport_immediate(
                egui::ViewportId(Id::new(("dialog", dialog.session, dialog.request))),
                egui::ViewportBuilder::default()
                    .with_title(&dialog.title)
                    .with_inner_size([420.0, 180.0])
                    .with_resizable(true),
                |ctx, _| {
                    closed = ctx.input(|i| i.viewport().close_requested());
                    egui::CentralPanel::default().show(ctx, |ui| {
                        egui::ScrollArea::vertical()
                            .max_height(ui.available_height() - 40.0)
                            .show(ui, |ui| {
                                ui.label(&dialog.text);
                            });
                        ui.separator();
                        ui.horizontal(|ui| {
                            for button in dialog.buttons.choices() {
                                if ui.button(button.to_uppercase()).clicked() {
                                    choice = Some((*button).to_owned());
                                }
                            }
                        });
                    });
                },
            );
            if closed {
                self.stop();
            } else if let Some(choice) = choice {
                let result = dialog.reply.try_send(choice).map_err(|e| e.to_string());
                self.dialog = None;
                self.report(result);
            }
        }
    }
}

impl eframe::App for AdfApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll(ctx);
        if ctx.input(|i| i.viewport().close_requested()) && !self.closing {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.request_action(Action::Close, ctx);
        }
        self.toolbar(ctx);
        self.tree(ctx);
        if self.tab == Tab::Design {
            self.inspector(ctx);
        }
        egui::CentralPanel::default().show(ctx, |ui| match self.tab {
            Tab::Xml => {
                ui.horizontal(|ui| {
                    if ui.button("Apply / validate").clicked() {
                        let result = self.editor.apply();
                        self.report(result);
                    }
                    if ui.button("Discard unapplied changes").clicked() {
                        self.editor.source = self.editor.applied_source.clone();
                        self.editor.error = None;
                    }
                });
                egui::ScrollArea::both().show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut self.editor.source)
                            .code_editor()
                            .desired_width(f32::INFINITY)
                            .desired_rows(30)
                            .char_limit(document::MAX_FILE_BYTES as usize),
                    );
                });
            }
            Tab::Design => self.designer(ui),
            Tab::Preview => {
                if let Some(mut form) = self.editor.selected_form().cloned() {
                    let size = Vec2::new(form.property.width, form.property.height);
                    egui::ScrollArea::both().show(ui, |ui| {
                        draw_form(ui, &mut form, false, size);
                    });
                }
            }
        });
        self.runtime_windows(ctx);
        if let Some(action) = self.pending.clone() {
            egui::Modal::new(Id::new("unsaved")).show(ctx, |ui| {
                ui.heading("Unsaved changes");
                ui.label("Save this project before continuing?");
                ui.horizontal(|ui| {
                    if ui.button("Save").clicked() && self.save_file(false) {
                        self.pending = None;
                        self.execute_action(action.clone(), ctx);
                    }
                    if ui.button("Discard").clicked() {
                        self.pending = None;
                        self.execute_action(action, ctx);
                    }
                    if ui.button("Cancel").clicked() {
                        self.pending = None;
                    }
                });
            });
        }
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, "preferences", &self.preferences);
    }
}

impl Drop for AdfApp {
    fn drop(&mut self) {
        self.stop();
        for session in &mut self.retired {
            session.stop();
            session.join();
        }
    }
}

struct WidgetEvent {
    name: String,
    change: bool,
    property: Property,
}

pub fn placements(form: &Control, size: Vec2) -> Vec<Rect> {
    let p = &form.property;
    let items = &form.controls.items;
    let padding = if p.layout == Layout::Absolute {
        0.0
    } else {
        p.padding
    };
    let mut offset = Vec2::splat(padding);
    let expands = items.iter().filter(|c| c.property.expand).count();
    let fixed: f32 = items
        .iter()
        .filter(|c| !c.property.expand)
        .map(|c| c.property.width)
        .sum();
    let free_width =
        (size.x - 2.0 * padding - fixed - p.spacing * items.len().saturating_sub(1) as f32)
            .max(1.0);
    items
        .iter()
        .map(|c| {
            let c = &c.property;
            let (position, width) = match p.layout {
                Layout::Absolute => (Vec2::new(c.left, c.top), c.width),
                Layout::Vertical => (
                    offset,
                    if c.expand {
                        (size.x - 2.0 * padding).max(1.0)
                    } else {
                        c.width
                    },
                ),
                Layout::Horizontal => (
                    offset,
                    if c.expand && expands > 0 {
                        free_width / expands as f32
                    } else {
                        c.width
                    },
                ),
            };
            let rect = Rect::from_min_size(
                Pos2::new(position.x, position.y),
                Vec2::new(width, c.height),
            );
            match p.layout {
                Layout::Vertical => offset.y += c.height + p.spacing,
                Layout::Horizontal => offset.x += width + p.spacing,
                Layout::Absolute => {}
            }
            rect
        })
        .collect()
}

fn draw_form(ui: &mut egui::Ui, form: &mut Control, enabled: bool, size: Vec2) -> Vec<WidgetEvent> {
    let positions = placements(form, size);
    let content = positions
        .iter()
        .fold(size, |size, rect| size.max(rect.max.to_vec2()));
    let mut events = vec![];
    let (canvas, _) = ui.allocate_exact_size(content, Sense::hover());
    ui.scope_builder(egui::UiBuilder::new().max_rect(canvas), |ui| {
        ui.add_enabled_ui(enabled, |ui| {
            for (c, rect) in form.controls.items.iter_mut().zip(positions) {
                let rect = rect.translate(canvas.min.to_vec2());
                let response = draw_control(ui, c, rect);
                let change = c.property.kind != Kind::Button;
                if (change && response.changed()) || (!change && response.clicked()) {
                    events.push(WidgetEvent {
                        name: c.property.name.clone(),
                        change,
                        property: c.property.clone(),
                    });
                }
            }
        });
    });
    events
}

fn draw_control(ui: &mut egui::Ui, c: &mut Control, rect: Rect) -> Response {
    let p = &mut c.property;
    ui.push_id(&p.name, |ui| match p.kind {
        Kind::Button => ui.put(rect, egui::Button::new(&p.text)),
        Kind::Label | Kind::Form => ui.put(rect, egui::Label::new(&p.text)),
        Kind::Textbox => ui.put(
            rect,
            egui::TextEdit::singleline(&mut p.text).char_limit(crate::model::MAX_TEXT),
        ),
        Kind::Checkbox => ui.put(rect, egui::Checkbox::new(&mut p.checked, &p.text)),
        Kind::Combobox => {
            let before = p.selected;
            let mut response = ui
                .scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                    egui::ComboBox::from_id_salt("value")
                        .width(rect.width())
                        .selected_text(p.options.items.get(p.selected).map_or("", String::as_str))
                        .show_ui(ui, |ui| {
                            for (index, option) in p.options.items.iter().enumerate() {
                                ui.selectable_value(&mut p.selected, index, option);
                            }
                        })
                        .response
                })
                .inner;
            if p.selected != before {
                response.mark_changed();
            }
            response
        }
    })
    .inner
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adaptive_layout_uses_padding_spacing_and_available_width() {
        let mut form = Control::new(Kind::Form, "MAIN".into());
        form.property.layout = Layout::Vertical;
        let mut first = Control::new(Kind::Button, "A".into());
        first.property.expand = true;
        form.controls.items = vec![first, Control::new(Kind::Button, "B".into())];
        let rects = placements(&form, Vec2::new(400.0, 200.0));
        assert_eq!(rects[0].width(), 384.0);
        assert_eq!(rects[1].top(), 48.0);
        form.property.layout = Layout::Absolute;
        form.controls.items[0].property.left = 10.0;
        assert_eq!(placements(&form, Vec2::new(400.0, 200.0))[0].left(), 10.0);
    }

    fn designer_app() -> AdfApp {
        AdfApp {
            editor: Editor::default(),
            file: None,
            preferences: Preferences::default(),
            tab: Tab::Design,
            session: None,
            retired: vec![],
            snapshot: None,
            dialog: None,
            errors: vec![],
            pending: None,
            closing: false,
            rename_for: String::new(),
            rename: String::new(),
            drag: None,
        }
    }

    fn frame(
        app: &mut AdfApp,
        ctx: &egui::Context,
        events: Vec<egui::Event>,
        tree: bool,
    ) -> egui::FullOutput {
        ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1100.0, 750.0))),
                events,
                ..Default::default()
            },
            |ctx| {
                if tree {
                    app.tree(ctx);
                }
                egui::CentralPanel::default().show(ctx, |ui| app.designer(ui));
            },
        )
    }

    fn pointer(position: Pos2, pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos: position,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        }
    }

    fn selected_rect(output: &egui::FullOutput) -> Rect {
        output
            .shapes
            .iter()
            .find_map(|shape| {
                if let egui::Shape::Rect(rect) = &shape.shape
                    && rect.stroke.color == Color32::LIGHT_BLUE
                {
                    Some(rect.rect)
                } else {
                    None
                }
            })
            .expect("Selected control outline")
    }

    #[test]
    fn designer_drag_accumulates_motion_across_frames_and_undo_restores_it() {
        let mut app = designer_app();
        app.editor.add(Kind::Button, None).unwrap();
        let name = app.editor.selected.clone();
        let before = app.editor.doc.clone();
        let ctx = egui::Context::default();
        frame(&mut app, &ctx, vec![], false);
        let output = frame(&mut app, &ctx, vec![], false);
        let start = selected_rect(&output).min + Vec2::splat(20.0);
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::PointerMoved(start), pointer(start, true)],
            false,
        );
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::PointerMoved(start + Vec2::new(20.0, 10.0))],
            false,
        );
        let end = start + Vec2::new(40.0, 30.0);
        frame(&mut app, &ctx, vec![egui::Event::PointerMoved(end)], false);
        frame(&mut app, &ctx, vec![pointer(end, false)], false);
        assert_eq!(
            app.editor.doc.find(&name).unwrap().property.left,
            before.find(&name).unwrap().property.left + 40.0
        );
        assert_eq!(
            app.editor.doc.find(&name).unwrap().property.top,
            before.find(&name).unwrap().property.top + 30.0
        );
        app.editor.undo();
        assert_eq!(app.editor.doc, before);
    }

    #[test]
    fn palette_drag_creates_a_control_at_the_drop_position() {
        let mut app = designer_app();
        app.editor.add(Kind::Button, None).unwrap();
        let ctx = egui::Context::default();
        let before = app.editor.doc.clone();
        frame(&mut app, &ctx, vec![], true);
        let output = frame(&mut app, &ctx, vec![], true);
        let start = output
            .shapes
            .iter()
            .find_map(|shape| {
                if let egui::Shape::Text(text) = &shape.shape
                    && text.galley.text() == "Add Label"
                {
                    Some(text.pos + text.galley.size() * 0.5)
                } else {
                    None
                }
            })
            .expect("Palette label");
        let drop = selected_rect(&output).min + Vec2::new(300.0, 200.0);
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::PointerMoved(start), pointer(start, true)],
            true,
        );
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::PointerMoved(start + Vec2::splat(20.0))],
            true,
        );
        frame(&mut app, &ctx, vec![egui::Event::PointerMoved(drop)], true);
        frame(&mut app, &ctx, vec![pointer(drop, false)], true);
        let label = app.editor.doc.controls.items[0]
            .controls
            .items
            .iter()
            .find(|c| c.property.kind == Kind::Label)
            .expect("Dropped label");
        let origin = &before.controls.items[0].controls.items[0].property;
        assert!((label.property.left - origin.left - 300.0).abs() < 0.01);
        assert!((label.property.top - origin.top - 200.0).abs() < 0.01);
        app.editor.undo();
        assert_eq!(app.editor.doc, before);
    }

    #[test]
    fn actual_widget_click_emits_one_event_and_busy_form_ignores_it() {
        let mut form = Control::new(Kind::Form, "MAIN".into());
        let mut button = Control::new(Kind::Button, "CLICK".into());
        button.property.left = 10.0;
        button.property.top = 10.0;
        form.controls.items.push(button);
        let ctx = egui::Context::default();
        let mut events = vec![];
        let mut render = |input, enabled| {
            let _ = ctx.run(
                egui::RawInput {
                    events: input,
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::splat(200.0))),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default()
                        .frame(egui::Frame::default())
                        .show(ctx, |ui| {
                            events.extend(draw_form(ui, &mut form, enabled, Vec2::splat(200.0)));
                        });
                },
            );
        };
        render(vec![], true);
        let pos = Pos2::new(30.0, 25.0);
        render(
            vec![egui::Event::PointerMoved(pos), pointer(pos, true)],
            true,
        );
        render(vec![pointer(pos, false)], true);
        render(vec![pointer(pos, true)], false);
        render(vec![pointer(pos, false)], false);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].name, "CLICK");
    }

    #[test]
    fn resize_handle_changes_dimensions_as_a_single_undo_gesture() {
        let mut app = designer_app();
        app.editor.add(Kind::Button, None).unwrap();
        let before = app.editor.doc.clone();
        let name = app.editor.selected.clone();
        let ctx = egui::Context::default();
        frame(&mut app, &ctx, vec![], false);
        let start = selected_rect(&frame(&mut app, &ctx, vec![], false)).max - Vec2::splat(5.0);
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::PointerMoved(start), pointer(start, true)],
            false,
        );
        let end = start + Vec2::new(40.0, 20.0);
        frame(&mut app, &ctx, vec![egui::Event::PointerMoved(end)], false);
        frame(&mut app, &ctx, vec![pointer(end, false)], false);
        let changed = &app.editor.doc.find(&name).unwrap().property;
        let original = &before.find(&name).unwrap().property;
        assert_eq!(changed.width, original.width + 40.0);
        assert_eq!(changed.height, original.height + 20.0);
        app.editor.undo();
        assert_eq!(app.editor.doc, before);
    }

    #[test]
    fn adaptive_drag_reorders_siblings_and_undo_restores_the_order() {
        let mut app = designer_app();
        app.editor.add(Kind::Button, None).unwrap();
        let first = app.editor.selected.clone();
        app.editor.add(Kind::Label, None).unwrap();
        let second = app.editor.selected.clone();
        app.editor
            .edit(|d| d.controls.items[0].property.layout = Layout::Vertical)
            .unwrap();
        let before = app.editor.doc.clone();
        let ctx = egui::Context::default();
        app.editor.selected = first;
        frame(&mut app, &ctx, vec![], false);
        let target = selected_rect(&frame(&mut app, &ctx, vec![], false)).center();
        app.editor.selected = second.clone();
        let start = selected_rect(&frame(&mut app, &ctx, vec![], false)).center();
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::PointerMoved(start), pointer(start, true)],
            false,
        );
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::PointerMoved(target)],
            false,
        );
        frame(&mut app, &ctx, vec![pointer(target, false)], false);
        assert_eq!(
            app.editor.doc.controls.items[0].controls.items[0]
                .property
                .name,
            second
        );
        app.editor.undo();
        assert_eq!(app.editor.doc, before);
    }
}
