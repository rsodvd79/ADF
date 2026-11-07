use crate::{
    document,
    model::{Control, Document, Kind},
};

pub struct Editor {
    pub doc: Document,
    pub source: String,
    pub applied_source: String,
    pub saved_source: String,
    pub error: Option<String>,
    pub selected: String,
    undo: Vec<Document>,
    redo: Vec<Document>,
    gesture: Option<Document>,
}

impl Default for Editor {
    fn default() -> Self {
        Self::from_doc(Document::default())
    }
}

impl Editor {
    pub fn from_doc(doc: Document) -> Self {
        let source = document::encode(&doc).unwrap_or_default();
        Self {
            selected: doc.property.start.clone(),
            doc,
            applied_source: source.clone(),
            saved_source: source.clone(),
            source,
            error: None,
            undo: vec![],
            redo: vec![],
            gesture: None,
        }
    }

    pub fn pending(&self) -> bool {
        self.source != self.applied_source
    }
    pub fn dirty(&self) -> bool {
        self.source != self.saved_source
    }
    pub fn apply(&mut self) -> Result<(), String> {
        let doc = document::parse(&self.source)?;
        if doc != self.doc {
            self.record();
        }
        self.doc = doc;
        if self.doc.find(&self.selected).is_none() {
            self.selected = self.doc.property.start.clone();
        }
        self.applied_source = self.source.clone();
        self.error = None;
        Ok(())
    }
    pub fn edit(&mut self, edit: impl FnOnce(&mut Document)) -> Result<(), String> {
        if self.pending() {
            return Err("Apply or discard XML changes before using the designer".into());
        }
        let mut candidate = self.doc.clone();
        edit(&mut candidate);
        let source = document::encode(&candidate)?;
        if candidate != self.doc {
            self.record();
            self.doc = candidate;
            self.source = source.clone();
            self.applied_source = source;
            if self.doc.find(&self.selected).is_none() {
                self.selected = self.doc.property.start.clone();
            }
        }
        self.error = None;
        Ok(())
    }
    pub fn begin_gesture(&mut self) {
        if self.gesture.is_none() {
            self.gesture = Some(self.doc.clone());
        }
    }
    pub fn update_gesture(&mut self, edit: impl FnOnce(&mut Document)) -> Result<(), String> {
        let mut candidate = self.doc.clone();
        edit(&mut candidate);
        let source = document::encode(&candidate)?;
        self.doc = candidate;
        self.source = source.clone();
        self.applied_source = source;
        Ok(())
    }
    pub fn end_gesture(&mut self) {
        if let Some(before) = self.gesture.take()
            && before != self.doc
        {
            if self.undo.len() == 32 {
                self.undo.remove(0);
            }
            self.undo.push(before);
            self.redo.clear();
        }
    }
    pub fn selected_form(&self) -> Option<&Control> {
        self.doc.controls.items.iter().find(|f| {
            f.property.name == self.selected
                || f.controls
                    .items
                    .iter()
                    .any(|c| c.property.name == self.selected)
        })
    }
    pub fn add(&mut self, kind: Kind, position: Option<(f32, f32)>) -> Result<(), String> {
        let name = self.doc.unique_name(&format!("{kind:?}").to_uppercase());
        let parent = self.selected_form().map(|f| f.property.name.clone());
        self.edit(|doc| {
            let mut c = Control::new(kind, name.clone());
            if let Some((left, top)) = position {
                c.property.left = left.clamp(0.0, 16_384.0);
                c.property.top = top.clamp(0.0, 16_384.0);
            }
            if kind == Kind::Form {
                doc.controls.items.push(c);
            } else if let Some(parent) = &parent
                && let Some(form) = doc.find_mut(parent)
            {
                form.controls.items.push(c);
            }
        })?;
        self.selected = name;
        Ok(())
    }
    pub fn delete_selected(&mut self) -> Result<(), String> {
        let selected = self.selected.clone();
        if selected == self.doc.property.start {
            return Err("Choose another STARTCONTROL before deleting this FORM".into());
        }
        self.edit(|doc| {
            doc.controls.items.retain(|f| f.property.name != selected);
            for f in &mut doc.controls.items {
                f.controls.items.retain(|c| c.property.name != selected);
            }
        })?;
        self.selected = self.doc.property.start.clone();
        Ok(())
    }
    pub fn duplicate_selected(&mut self) -> Result<(), String> {
        let mut copy = self
            .doc
            .find(&self.selected)
            .cloned()
            .ok_or("Select a control")?;
        let parent = self.selected_form().map(|f| f.property.name.clone());
        let mut naming = self.doc.clone();
        copy.property.name = naming.unique_name(&copy.property.name);
        let name = copy.property.name.clone();
        naming
            .controls
            .items
            .push(Control::new(Kind::Form, name.clone()));
        for c in &mut copy.controls.items {
            c.property.name = naming.unique_name(&c.property.name);
            naming
                .controls
                .items
                .push(Control::new(Kind::Form, c.property.name.clone()));
        }
        self.edit(|doc| {
            if copy.property.kind == Kind::Form {
                doc.controls.items.push(copy);
            } else if let Some(parent) = parent
                && let Some(form) = doc.find_mut(&parent)
            {
                form.controls.items.push(copy);
            }
        })?;
        self.selected = name;
        Ok(())
    }
    pub fn rename_selected(&mut self, name: String) -> Result<(), String> {
        let old = self.selected.clone();
        self.edit(|doc| {
            if let Some(c) = doc.find_mut(&old) {
                c.property.name = name.clone();
            }
            if doc.property.start == old {
                doc.property.start = name.clone();
            }
        })?;
        self.selected = name;
        Ok(())
    }
    pub fn record(&mut self) {
        // ponytail: bounded document snapshots; use operation history if projects grow.
        if self.undo.len() == 32 {
            self.undo.remove(0);
        }
        self.undo.push(self.doc.clone());
        self.redo.clear();
    }
    pub fn sync(&mut self) -> Result<(), String> {
        let source = document::encode(&self.doc)?;
        self.source = source.clone();
        self.applied_source = source;
        if self.doc.find(&self.selected).is_none() {
            self.selected = self.doc.property.start.clone();
        }
        self.error = None;
        Ok(())
    }
    pub fn undo(&mut self) {
        if let Some(doc) = self.undo.pop() {
            self.redo.push(self.doc.clone());
            self.doc = doc;
            if let Err(e) = self.sync() {
                self.error = Some(e);
            }
        }
    }

    pub fn redo(&mut self) {
        if let Some(doc) = self.redo.pop() {
            self.undo.push(self.doc.clone());
            self.doc = doc;
            if let Err(e) = self.sync() {
                self.error = Some(e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_xml_does_not_replace_last_valid_model() {
        let mut editor = Editor::default();
        let before = editor.doc.clone();
        editor.source = "<ADF>".into();
        assert!(editor.apply().is_err());
        assert_eq!(editor.doc, before);
        assert!(editor.add(Kind::Button, None).is_err());
    }

    #[test]
    fn visual_edit_roundtrips_and_undo_redo_restore_the_model() {
        let mut editor = Editor::default();
        editor.add(Kind::Button, None).unwrap();
        let added = editor.doc.clone();
        assert_eq!(document::parse(&editor.source).unwrap(), added);
        editor.undo();
        assert_eq!(editor.doc, Document::default());
        editor.redo();
        assert_eq!(editor.doc, added);
    }

    #[test]
    fn duplicate_form_renames_children_and_rename_updates_start() {
        let mut editor = Editor::default();
        editor.add(Kind::Button, None).unwrap();
        editor.selected = "MAIN".into();
        editor.duplicate_selected().unwrap();
        editor.doc.validate().unwrap();
        editor.selected = "MAIN".into();
        editor.rename_selected("RENAMED".into()).unwrap();
        assert_eq!(editor.doc.property.start, "RENAMED");
    }

    #[test]
    fn drag_is_a_single_undo_gesture() {
        let mut editor = Editor::default();
        editor.add(Kind::Button, None).unwrap();
        let name = editor.selected.clone();
        let before = editor.doc.clone();
        editor.begin_gesture();
        for left in [20.0, 40.0, 60.0] {
            editor
                .update_gesture(|d| d.find_mut(&name).unwrap().property.left = left)
                .unwrap();
        }
        editor.end_gesture();
        editor.undo();
        assert_eq!(editor.doc, before);
    }
}
