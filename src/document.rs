use crate::model::{Document, MAX_DEPTH};
use quick_xml::{Reader, events::Event};
use std::{
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
};

pub const MAX_FILE_BYTES: u64 = 5 * 1_048_576;

pub fn parse(source: &str) -> Result<Document, String> {
    if source.len() as u64 > MAX_FILE_BYTES {
        return Err("XML exceeds 5 MiB".into());
    }
    let source = source.trim_start_matches('\u{feff}');
    let mut reader = Reader::from_str(source);
    let mut depth = 0;
    let mut roots = 0;
    loop {
        match reader.read_event() {
            Ok(Event::DocType(_)) => {
                return Err("DTD and external entities are not supported".into());
            }
            Ok(Event::Start(e)) => {
                if depth == 0 {
                    roots += 1;
                    if roots != 1 || e.name().as_ref() != "ADF" {
                        return Err("Expected exactly one ADF root".into());
                    }
                }
                depth += 1;
                if depth > MAX_DEPTH {
                    return Err("XML nesting exceeds 32 elements".into());
                }
            }
            Ok(Event::Empty(e)) if depth == 0 => {
                roots += 1;
                if roots != 1 || e.name().as_ref() != "ADF" {
                    return Err("Expected exactly one ADF root".into());
                }
            }
            Ok(Event::End(_)) => depth = depth.saturating_sub(1),
            Ok(Event::Text(text)) if depth == 0 && !text.as_ref().trim().is_empty() => {
                return Err("Text outside the ADF root is not supported".into());
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(format!("XML byte {}: {e}", reader.error_position())),
            _ => {}
        }
    }
    if roots != 1 || depth != 0 {
        return Err("Incomplete ADF document".into());
    }
    let doc: Document = quick_xml::de::from_str(source).map_err(|e| format!("XML: {e}"))?;
    doc.validate()?;
    Ok(doc)
}

pub fn encode(doc: &Document) -> Result<String, String> {
    doc.validate()?;
    let mut source = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    let mut serializer = quick_xml::se::Serializer::new(&mut source);
    serializer.indent(' ', 2);
    serde::Serialize::serialize(doc, serializer).map_err(|e| e.to_string())?;
    if source.len() as u64 > MAX_FILE_BYTES {
        return Err("Serialized XML exceeds 5 MiB".into());
    }
    Ok(source)
}

pub fn load(path: &Path) -> Result<String, String> {
    let file = File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut bytes = Vec::new();
    file.take(MAX_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err("XML exceeds 5 MiB".into());
    }
    String::from_utf8(bytes).map_err(|e| format!("Expected UTF-8 XML: {e}"))
}

pub fn save(path: &Path, source: &str) -> Result<(), String> {
    parse(source)?;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut temp = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    temp.write_all(source.as_bytes())
        .map_err(|e| e.to_string())?;
    temp.as_file().sync_all().map_err(|e| e.to_string())?;
    temp.persist(path)
        .map_err(|e| format!("Cannot replace {}: {e}", path.display()))?;
    Ok(())
}

#[derive(Clone, Debug)]
pub struct OpenFile {
    pub path: PathBuf,
    pub disk_source: String,
}

impl OpenFile {
    pub fn changed_on_disk(&self) -> bool {
        load(&self.path).map_or(true, |s| s != self.disk_source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Control, Kind};
    use std::fs;

    #[test]
    fn legacy_ciao_preserves_vbs_without_marking_it_executable() {
        let doc = parse(include_str!("../ADF/CIAO.XML")).unwrap();
        assert!(doc.is_legacy() && doc.find("OK").unwrap().events.click.contains("MsgBox"));
    }

    #[test]
    fn round_trip_preserves_multiline_script_and_unicode() {
        let mut doc = Document {
            module: "let greeting = \"Citt\u{e0} < & >\";\nlet n = 2;".into(),
            ..Default::default()
        };
        doc.controls.items[0]
            .controls
            .items
            .push(Control::new(Kind::Combobox, "CHOICE".into()));
        assert_eq!(parse(&encode(&doc).unwrap()).unwrap(), doc);
    }

    #[test]
    fn rejects_dtd_and_wrong_root() {
        for xml in [
            "<!DOCTYPE ADF><ADF/>",
            "<OTHER/>",
            "<ADF/><ADF/>",
            "junk<ADF/>",
        ] {
            assert!(parse(xml).is_err(), "{xml}");
        }
    }

    #[test]
    fn legacy_serializer_namespaces_and_empty_elements_are_supported() {
        let source = include_str!("../ADF/CIAO.XML").replace("<ADF>", "<ADF xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" xmlns:xsd=\"http://www.w3.org/2001/XMLSchema\">");
        assert!(parse(&source).unwrap().is_legacy());
    }

    #[test]
    fn malformed_versions_and_unknown_fields_are_rejected() {
        let source = encode(&Document::default()).unwrap();
        for invalid in [
            source.replace("VERSION=\"2\"", "VERSION=\"3\""),
            source.replace("SCRIPT_LANGUAGE=\"rhai\"", "SCRIPT_LANGUAGE=\"vbscript\""),
            source.replace("</ADF>", "<UNKNOWN/></ADF>"),
            source.replace("640", "NaN"),
        ] {
            assert!(parse(&invalid).is_err());
        }
    }
    #[test]
    fn rejects_invalid_model_and_deep_xml() {
        let mut doc = Document::default();
        doc.controls.items.push(doc.controls.items[0].clone());
        assert!(encode(&doc).is_err());
        let nested = format!("{}{}", "<ADF>".repeat(33), "</ADF>".repeat(33));
        assert!(parse(&nested).is_err());
    }

    #[test]
    fn failed_save_keeps_existing_file_and_successful_save_replaces_it() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("existing.xml");
        let source = encode(&Document::default()).unwrap();
        save(&path, &source).unwrap();
        assert!(save(&path, "<invalid/>").is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), source);
        let mut doc = Document::default();
        doc.controls.items[0].property.text = "Changed".into();
        save(&path, &encode(&doc).unwrap()).unwrap();
        assert_eq!(parse(&fs::read_to_string(path).unwrap()).unwrap(), doc);
    }

    #[test]
    fn missing_load_does_not_create_a_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("missing.xml");
        assert!(load(&path).is_err());
        assert!(!path.exists());
    }
}
