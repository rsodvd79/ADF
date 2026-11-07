use crate::{runtime::Buttons, ui::AdfApp};
use eframe::egui;
use std::{
    cell::RefCell,
    path::PathBuf,
    rc::Rc,
    time::{Duration, Instant},
};

struct Smoke {
    app: AdfApp,
    phase: u8,
    start: Instant,
    outcome: Rc<RefCell<Result<(), String>>>,
}

impl Smoke {
    fn event(&mut self, name: &str, change: bool, value: Option<&str>) -> Result<(), String> {
        let mut property = self
            .app
            .snapshot
            .as_ref()
            .ok_or("No snapshot")?
            .doc
            .find(name)
            .ok_or("Control not rendered")?
            .property
            .clone();
        if let Some(value) = value {
            property.text = value.into();
        }
        self.app
            .session
            .as_mut()
            .ok_or("No worker")?
            .event(name, change, property)
    }

    fn step(&mut self, ctx: &egui::Context) -> Result<bool, String> {
        if self.start.elapsed() > Duration::from_secs(20) {
            return Err(format!("Native smoke timed out at phase {}", self.phase));
        }
        if let Some(error) = self.app.errors.last() {
            return Err(error.clone());
        }
        let ready = self.app.session.as_ref().is_some_and(|s| !s.busy);
        match self.phase {
            0 => {
                self.app.run(ctx);
                self.phase = 1;
            }
            1 if ready => {
                self.event("OK", false, None)?;
                self.phase = 2;
            }
            2 if self.app.dialog.is_some() => {
                let dialog = self.app.dialog.take().ok_or("Missing CIAO dialog")?;
                if dialog.title != "DVD" || dialog.text != "CIAO" || dialog.buttons != Buttons::Ok {
                    return Err("Incorrect CIAO dialog".into());
                }
                dialog
                    .reply
                    .try_send("ok".into())
                    .map_err(|e| e.to_string())?;
                self.phase = 3;
            }
            3 if ready => {
                let doc = &self.app.snapshot.as_ref().ok_or("No CIAO snapshot")?.doc;
                if doc.find("OK").ok_or("No OK button")?.property.text != "SOMMA = 2"
                    || doc.find("CIAO").ok_or("No CIAO viewport")?.property.text != "ADF"
                {
                    return Err("CIAO result mismatch".into());
                }
                self.app
                    .open(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/showcase.xml"));
                self.app.run(ctx);
                self.phase = 4;
            }
            4 if ready => {
                self.event("OPEN", false, None)?;
                self.phase = 5;
            }
            5 if ready => {
                if !self
                    .app
                    .snapshot
                    .as_ref()
                    .ok_or("No showcase snapshot")?
                    .visible
                    .contains("DETAILS")
                {
                    return Err("Secondary viewport was not opened".into());
                }
                self.event("NAME", true, Some("Rust"))?;
                self.phase = 6;
            }
            6 if ready => {
                if self
                    .app
                    .snapshot
                    .as_ref()
                    .ok_or("No snapshot")?
                    .doc
                    .find("GREETING")
                    .ok_or("No label")?
                    .property
                    .text
                    != "Hello Rust"
                {
                    return Err("CHANGE did not update the label".into());
                }
                self.event("COUNT", false, None)?;
                self.phase = 7;
            }
            7 if self.app.dialog.is_some() => {
                let dialog = self.app.dialog.take().ok_or("Missing confirm dialog")?;
                if dialog.buttons != Buttons::YesNo {
                    return Err("Incorrect confirmation buttons".into());
                }
                dialog
                    .reply
                    .try_send("yes".into())
                    .map_err(|e| e.to_string())?;
                self.phase = 8;
            }
            8 if ready => {
                if self
                    .app
                    .snapshot
                    .as_ref()
                    .ok_or("No snapshot")?
                    .doc
                    .find("GREETING")
                    .ok_or("No label")?
                    .property
                    .text
                    != "Count: 1"
                {
                    return Err("Persistent module state mismatch".into());
                }
                self.app
                    .session
                    .as_mut()
                    .ok_or("No worker")?
                    .hide_form("DETAILS")?;
                self.phase = 9;
            }
            9 if ready => {
                if self
                    .app
                    .snapshot
                    .as_ref()
                    .ok_or("No snapshot")?
                    .visible
                    .contains("DETAILS")
                {
                    return Err("Closed viewport reappeared".into());
                }
                return Ok(true);
            }
            _ => {}
        }
        Ok(false)
    }
}

impl eframe::App for Smoke {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.app.update(ctx, frame);
        let result = self.step(ctx);
        if !matches!(result, Ok(false)) {
            *self.outcome.borrow_mut() = result.map(|_| ());
            self.app.stop();
            self.app.closing = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        } else {
            ctx.request_repaint_after(Duration::from_millis(10));
        }
    }
}

pub fn run() -> Result<(), String> {
    let outcome = Rc::new(RefCell::new(Err(
        "Native window closed before smoke completed".into(),
    )));
    let result = outcome.clone();
    eframe::run_native(
        "ADF native smoke",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([1100.0, 750.0]),
            persist_window: false,
            ..Default::default()
        },
        Box::new(move |cc| {
            Ok(Box::new(Smoke {
                app: AdfApp::new(
                    cc,
                    Some(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/ciao.xml")),
                ),
                phase: 0,
                start: Instant::now(),
                outcome: result,
            }))
        }),
    )
    .map_err(|e| e.to_string())?;
    outcome.borrow().clone()
}
