use crate::model::{Document, Kind, MAX_TEXT, Property};
use rhai::{AST, Dynamic, Engine, EvalAltResult, ImmutableString, Position, Scope};
use std::{
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet},
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{self, Receiver, SyncSender, TrySendError},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

const ACTIVE_LIMIT: Duration = Duration::from_secs(1);
const MAX_OPERATIONS: u64 = 100_000;
static NEXT_SESSION: AtomicU64 = AtomicU64::new(1);
type ScriptResult<T> = Result<T, Box<EvalAltResult>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Buttons {
    Ok,
    OkCancel,
    YesNo,
}

impl Buttons {
    pub fn choices(self) -> &'static [&'static str] {
        match self {
            Self::Ok => &["ok"],
            Self::OkCancel => &["ok", "cancel"],
            Self::YesNo => &["yes", "no"],
        }
    }
}

pub struct Dialog {
    pub session: u64,
    pub request: u64,
    pub title: String,
    pub text: String,
    pub buttons: Buttons,
    pub reply: SyncSender<String>,
}

#[derive(Clone)]
pub struct Snapshot {
    pub doc: Document,
    pub visible: HashSet<String>,
}

pub enum Update {
    State(Box<Snapshot>),
    Dialog(Dialog),
    Ready,
    Error(String),
    Log(String),
    Finished,
}

struct EventRequest {
    name: String,
    change: bool,
    property: Property,
}

enum Command {
    Event(Box<EventRequest>),
    Hide(String),
}

pub struct Session {
    pub id: u64,
    pub busy: bool,
    commands: Option<SyncSender<Command>>,
    pub updates: Receiver<Update>,
    cancel: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl Session {
    pub fn start(doc: Document, wake: impl Fn() + Send + 'static) -> Result<Self, String> {
        doc.validate()?;
        if doc.is_legacy() {
            return Err("Legacy VBScript is not executable. Convert every script and mark the XML as v2/Rhai.".into());
        }
        let id = NEXT_SESSION.fetch_add(1, Ordering::Relaxed);
        let (tx, updates) = mpsc::sync_channel(16);
        let (commands, rx) = mpsc::sync_channel(1);
        let cancel = Arc::new(AtomicBool::new(false));
        let flag = cancel.clone();
        let worker = thread::Builder::new()
            .name(format!("adf-script-{id}"))
            .spawn(move || {
                let world = Rc::new(RefCell::new(World {
                    visible: HashSet::from([doc.property.start.clone()]),
                    doc,
                    tx,
                    cancel: flag,
                    wake: Box::new(wake),
                    id,
                    request: 0,
                    paused: Rc::new(Cell::new(Duration::ZERO)),
                    state: HashMap::new(),
                }));
                if let Err(e) = run_worker(world.clone(), rx) {
                    let _ = world.borrow().send(Update::Error(e));
                }
                let _ = world.borrow().send(Update::Finished);
            })
            .map_err(|e| e.to_string())?;
        Ok(Self {
            id,
            busy: true,
            commands: Some(commands),
            updates,
            cancel,
            worker: Some(worker),
        })
    }

    pub fn event(&mut self, name: &str, change: bool, property: Property) -> Result<(), String> {
        if self.busy {
            return Err("An event is already running".into());
        }
        let Some(tx) = &self.commands else {
            return Err("Execution has stopped".into());
        };
        tx.try_send(Command::Event(Box::new(EventRequest {
            name: name.into(),
            change,
            property,
        })))
        .map_err(|e| e.to_string())?;
        self.busy = true;
        Ok(())
    }

    pub fn hide_form(&mut self, name: &str) -> Result<(), String> {
        if self.busy {
            return Err("An event is already running".into());
        }
        self.commands
            .as_ref()
            .ok_or("Execution has stopped")?
            .try_send(Command::Hide(name.into()))
            .map_err(|e| e.to_string())?;
        self.busy = true;
        Ok(())
    }

    pub fn stop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
        self.commands.take();
    }

    pub fn is_finished(&self) -> bool {
        self.worker.as_ref().is_none_or(JoinHandle::is_finished)
    }

    pub fn join(&mut self) {
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.stop();
    }
}

struct World {
    doc: Document,
    visible: HashSet<String>,
    tx: SyncSender<Update>,
    cancel: Arc<AtomicBool>,
    wake: Box<dyn Fn()>,
    id: u64,
    request: u64,
    paused: Rc<Cell<Duration>>,
    state: HashMap<String, Dynamic>,
}

fn error(message: impl Into<String>) -> Box<EvalAltResult> {
    EvalAltResult::ErrorRuntime(message.into().into(), Position::NONE).into()
}

impl World {
    fn send(&self, mut update: Update) -> Result<(), String> {
        loop {
            if self.cancel.load(Ordering::Relaxed) {
                return Err("Execution cancelled".into());
            }
            match self.tx.try_send(update) {
                Ok(()) => {
                    (self.wake)();
                    return Ok(());
                }
                Err(TrySendError::Full(value)) => {
                    update = value;
                    thread::sleep(Duration::from_millis(5));
                }
                Err(TrySendError::Disconnected(_)) => return Err("Editor closed".into()),
            }
        }
    }

    fn publish(&self) -> Result<(), String> {
        self.send(Update::State(Box::new(Snapshot {
            doc: self.doc.clone(),
            visible: self.visible.clone(),
        })))
    }

    fn dialog(
        &mut self,
        title: &str,
        text: &str,
        buttons: Buttons,
    ) -> ScriptResult<ImmutableString> {
        if title.len() > MAX_TEXT || text.len() > MAX_TEXT {
            return Err(error("Dialog text exceeds 1 MiB"));
        }
        self.publish().map_err(error)?;
        self.request += 1;
        let (reply, rx) = mpsc::sync_channel(1);
        self.send(Update::Dialog(Dialog {
            session: self.id,
            request: self.request,
            title: title.into(),
            text: text.into(),
            buttons,
            reply,
        }))
        .map_err(error)?;
        let wait = Instant::now();
        let result = loop {
            if self.cancel.load(Ordering::Relaxed) {
                break Err(error("Execution cancelled while waiting for a dialog"));
            }
            match rx.recv_timeout(Duration::from_millis(10)) {
                Ok(choice) if buttons.choices().contains(&choice.as_str()) => {
                    break Ok(choice.into());
                }
                Ok(_) => break Err(error("Invalid dialog response")),
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    break Err(error("Dialog closed without a response"));
                }
            }
        };
        self.paused.set(self.paused.get() + wait.elapsed());
        result
    }
}

fn run_worker(world: Rc<RefCell<World>>, rx: Receiver<Command>) -> Result<(), String> {
    let mut engine = Engine::new();
    engine.disable_symbol("eval");
    engine.set_max_operations(MAX_OPERATIONS);
    engine.set_max_call_levels(64);
    engine.set_max_expr_depths(64, 32);
    engine.set_max_string_size(MAX_TEXT);
    engine.set_max_array_size(10_000);
    engine.set_max_map_size(10_000);
    engine.set_max_variables(256);
    engine.set_max_functions(1_024);
    let clock = Rc::new(Cell::new(Instant::now()));
    let paused = world.borrow().paused.clone();
    let cancel = world.borrow().cancel.clone();
    let start = clock.clone();
    engine.on_progress(move |_| {
        let active = start.get().elapsed().saturating_sub(paused.get());
        if cancel.load(Ordering::Relaxed) {
            Some("Execution cancelled".into())
        } else if active > ACTIVE_LIMIT {
            Some("One-second active execution limit exceeded".into())
        } else {
            None
        }
    });
    register_api(&mut engine, &world);
    let module = world.borrow().doc.module.clone();
    let module_ast = engine
        .compile(&module)
        .map_err(|e| format!("MODULO: {e}"))?;
    let functions = module_ast.clone_functions_only();
    let mut handlers: HashMap<(String, bool), AST> = HashMap::new();
    {
        let w = world.borrow();
        for form in &w.doc.controls.items {
            for c in &form.controls.items {
                for (change, source) in [(false, &c.events.click), (true, &c.events.change)] {
                    if !source.trim().is_empty() {
                        let ast = engine.compile(source).map_err(|e| {
                            format!(
                                "CONTROL '{}' {}: {e}",
                                c.property.name,
                                if change { "CHANGE" } else { "CLICK" }
                            )
                        })?;
                        handlers.insert((c.property.name.clone(), change), functions.merge(&ast));
                    }
                }
            }
        }
    }
    let mut scope = Scope::new();
    clock.set(Instant::now());
    engine
        .run_ast_with_scope(&mut scope, &module_ast)
        .map_err(|e| format!("MODULO: {e}"))?;
    world.borrow().publish()?;
    world.borrow().send(Update::Ready)?;
    while let Ok(command) = rx.recv() {
        if world.borrow().cancel.load(Ordering::Relaxed) {
            break;
        }
        let event = match command {
            Command::Event(event) => event,
            Command::Hide(name) => {
                world.borrow_mut().visible.remove(&name);
                world.borrow().publish()?;
                world.borrow().send(Update::Ready)?;
                continue;
            }
        };
        if event.change {
            let mut w = world.borrow_mut();
            let mut candidate = w.doc.clone();
            let result = match candidate.find_mut(&event.name) {
                Some(c)
                    if c.property.name == event.property.name
                        && c.property.kind == event.property.kind =>
                {
                    match c.property.kind {
                        Kind::Textbox => c.property.text = event.property.text,
                        Kind::Checkbox => c.property.checked = event.property.checked,
                        Kind::Combobox => c.property.selected = event.property.selected,
                        _ => {}
                    }
                    candidate.validate()
                }
                _ => Err(format!("Unknown or mismatched control '{}'", event.name)),
            };
            if let Err(e) = result {
                w.send(Update::Error(e))?;
                w.send(Update::Ready)?;
                continue;
            }
            w.doc = candidate;
        }
        let scope_size = scope.len();
        world.borrow().paused.set(Duration::ZERO);
        clock.set(Instant::now());
        if let Some(ast) = handlers.get(&(event.name.clone(), event.change))
            && let Err(e) = engine.run_ast_with_scope(&mut scope, ast)
        {
            world.borrow().send(Update::Error(format!(
                "CONTROL '{}' {}: {e}",
                event.name,
                if event.change { "CHANGE" } else { "CLICK" }
            )))?;
        }
        scope.rewind(scope_size);
        world.borrow().publish()?;
        world.borrow().send(Update::Ready)?;
    }
    Ok(())
}

fn register_api(engine: &mut Engine, world: &Rc<RefCell<World>>) {
    let w = world.clone();
    engine.register_fn(
        "get_text",
        move |name: &str| -> ScriptResult<ImmutableString> {
            w.borrow()
                .doc
                .find(name)
                .map(|c| c.property.text.clone().into())
                .ok_or_else(|| error(format!("Unknown control '{name}'")))
        },
    );
    let w = world.clone();
    engine.register_fn(
        "set_text",
        move |name: &str, text: &str| -> ScriptResult<()> {
            if text.len() > MAX_TEXT {
                return Err(error("Text exceeds 1 MiB"));
            }
            let mut w = w.borrow_mut();
            let c = w
                .doc
                .find_mut(name)
                .ok_or_else(|| error(format!("Unknown control '{name}'")))?;
            c.property.text = text.into();
            Ok(())
        },
    );
    let w = world.clone();
    engine.register_fn("get_value", move |name: &str| -> ScriptResult<Dynamic> {
        let w = w.borrow();
        let c = w
            .doc
            .find(name)
            .ok_or_else(|| error(format!("Unknown control '{name}'")))?;
        match c.property.kind {
            Kind::Textbox => Ok(c.property.text.clone().into()),
            Kind::Checkbox => Ok(c.property.checked.into()),
            Kind::Combobox => Ok((c.property.selected as i64).into()),
            _ => Err(error("get_value requires TEXTBOX, CHECKBOX or COMBOBOX")),
        }
    });
    let w = world.clone();
    engine.register_fn(
        "set_value",
        move |name: &str, value: Dynamic| -> ScriptResult<()> {
            let mut w = w.borrow_mut();
            let c = w
                .doc
                .find_mut(name)
                .ok_or_else(|| error(format!("Unknown control '{name}'")))?;
            match c.property.kind {
                Kind::Textbox => {
                    let text = value
                        .try_cast::<ImmutableString>()
                        .ok_or_else(|| error("TEXTBOX requires a string"))?;
                    if text.len() > MAX_TEXT {
                        return Err(error("Text exceeds 1 MiB"));
                    }
                    c.property.text = text.into();
                }
                Kind::Checkbox => {
                    c.property.checked = value
                        .try_cast::<bool>()
                        .ok_or_else(|| error("CHECKBOX requires bool"))?
                }
                Kind::Combobox => {
                    let index = value
                        .try_cast::<i64>()
                        .ok_or_else(|| error("COMBOBOX requires an integer"))?;
                    if index < 0 || index as usize >= c.property.options.items.len() {
                        return Err(error("COMBOBOX index is out of range"));
                    }
                    c.property.selected = index as usize;
                }
                _ => return Err(error("set_value requires TEXTBOX, CHECKBOX or COMBOBOX")),
            }
            Ok(())
        },
    );
    for (function, visible) in [("show_form", true), ("close_form", false)] {
        let w = world.clone();
        engine.register_fn(function, move |name: &str| -> ScriptResult<()> {
            let mut w = w.borrow_mut();
            if w.doc
                .find(name)
                .is_none_or(|c| c.property.kind != Kind::Form)
            {
                return Err(error(format!("Unknown FORM '{name}'")));
            }
            if visible {
                w.visible.insert(name.into());
            } else {
                w.visible.remove(name);
            }
            Ok(())
        });
    }
    let w = world.clone();
    engine.register_fn("message", move |title: &str, text: &str| {
        w.borrow_mut().dialog(title, text, Buttons::Ok)
    });
    let w = world.clone();
    engine.register_fn(
        "confirm",
        move |title: &str, text: &str, buttons: &str| -> ScriptResult<ImmutableString> {
            let buttons = match buttons {
                "ok_cancel" => Buttons::OkCancel,
                "yes_no" => Buttons::YesNo,
                _ => return Err(error("confirm buttons must be 'ok_cancel' or 'yes_no'")),
            };
            w.borrow_mut().dialog(title, text, buttons)
        },
    );
    let w = world.clone();
    engine.register_fn("get_state", move |key: &str| -> Dynamic {
        w.borrow().state.get(key).cloned().unwrap_or(Dynamic::UNIT)
    });
    let w = world.clone();
    engine.register_fn(
        "set_state",
        move |key: &str, value: Dynamic| -> ScriptResult<()> {
            if key.len() > 128
                || !(value.is_unit()
                    || value.is_bool()
                    || value.is_int()
                    || value.is_float()
                    || value.is_string())
            {
                return Err(error(
                    "State keys must fit 128 bytes; values must be scalar",
                ));
            }
            if value.is_string() && value.clone_cast::<ImmutableString>().len() > MAX_TEXT {
                return Err(error("State text exceeds 1 MiB"));
            }
            let mut w = w.borrow_mut();
            if w.state.len() >= 256 && !w.state.contains_key(key) {
                return Err(error("State exceeds 256 keys"));
            }
            w.state.insert(key.into(), value);
            Ok(())
        },
    );
    let w = world.clone();
    engine.on_print(move |text| {
        let w = w.borrow();
        let _ = w.tx.try_send(Update::Log(text.into()));
        (w.wake)();
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Control;

    fn project(script: &str) -> Document {
        let mut doc = Document::default();
        let mut button = Control::new(Kind::Button, "OK".into());
        button.events.click = script.into();
        doc.controls.items[0].controls.items.push(button);
        doc
    }

    fn ready(session: &mut Session) -> (Option<Snapshot>, Vec<String>) {
        let mut state = None;
        let mut errors = vec![];
        loop {
            match session
                .updates
                .recv_timeout(Duration::from_secs(5))
                .unwrap()
            {
                Update::State(s) => state = Some(*s),
                Update::Error(e) => errors.push(e),
                Update::Log(_) => {}
                Update::Ready => {
                    session.busy = false;
                    return (state, errors);
                }
                Update::Finished => return (state, errors),
                Update::Dialog(_) => panic!("Unexpected dialog"),
            }
        }
    }

    #[test]
    fn module_functions_and_state_are_shared_across_forms_and_events() {
        let mut doc = project(
            "set_state(\"n\", get_state(\"n\") + 1); set_text(\"OK\", sum(get_state(\"n\"), 1).to_string());",
        );
        doc.module = "fn sum(a,b) { a+b } set_state(\"n\", 0);".into();
        let mut form = Control::new(Kind::Form, "OTHER".into());
        let mut button = Control::new(Kind::Button, "SECOND".into());
        button.events.click = "set_text(\"SECOND\", sum(get_state(\"n\"), 1).to_string());".into();
        form.controls.items.push(button);
        doc.controls.items.push(form);
        let property = doc.find("OK").unwrap().property.clone();
        let other = doc.find("SECOND").unwrap().property.clone();
        let mut s = Session::start(doc, || {}).unwrap();
        assert!(ready(&mut s).1.is_empty());
        for _ in 0..2 {
            s.event("OK", false, property.clone()).unwrap();
            assert!(ready(&mut s).1.is_empty());
        }
        s.event("SECOND", false, other).unwrap();
        let (state, errors) = ready(&mut s);
        assert!(errors.is_empty());
        assert_eq!(
            state.unwrap().doc.find("SECOND").unwrap().property.text,
            "3"
        );
        s.stop();
        s.join();
    }

    #[test]
    fn dialog_suspends_script_and_user_wait_does_not_consume_active_timeout() {
        let doc = project(
            "let choice = confirm(\"Title\", \"Question\", \"yes_no\"); set_text(\"OK\", choice);",
        );
        let property = doc.find("OK").unwrap().property.clone();
        let mut s = Session::start(doc, || {}).unwrap();
        ready(&mut s);
        s.event("OK", false, property).unwrap();
        let dialog = loop {
            match s.updates.recv_timeout(Duration::from_secs(5)).unwrap() {
                Update::Dialog(d) => break d,
                Update::State(state) => {
                    assert_eq!(state.doc.find("OK").unwrap().property.text, "OK")
                }
                _ => panic!("Unexpected update"),
            }
        };
        thread::sleep(Duration::from_millis(1_100));
        dialog.reply.send("yes".into()).unwrap();
        let (state, errors) = ready(&mut s);
        assert!(errors.is_empty());
        assert_eq!(state.unwrap().doc.find("OK").unwrap().property.text, "yes");
        s.stop();
        s.join();
    }

    #[test]
    fn stopping_a_dialog_terminates_without_running_following_instructions() {
        let doc = project("message(\"T\", \"Q\"); set_text(\"OK\", \"must not run\");");
        let property = doc.find("OK").unwrap().property.clone();
        let mut s = Session::start(doc, || {}).unwrap();
        ready(&mut s);
        s.event("OK", false, property).unwrap();
        let dialog = loop {
            if let Update::Dialog(d) = s.updates.recv_timeout(Duration::from_secs(5)).unwrap() {
                break d;
            }
        };
        s.stop();
        let limit = Instant::now() + Duration::from_secs(2);
        while !s.is_finished() && Instant::now() < limit {
            thread::sleep(Duration::from_millis(10));
        }
        assert!(s.is_finished());
        s.join();
        assert!(dialog.reply.try_send("ok".into()).is_err());
    }

    #[test]
    fn operation_limit_stops_infinite_loop_and_retains_prior_mutations() {
        let doc = project("set_text(\"OK\", \"before error\"); loop {}");
        let property = doc.find("OK").unwrap().property.clone();
        let mut s = Session::start(doc, || {}).unwrap();
        ready(&mut s);
        s.event("OK", false, property).unwrap();
        let (state, errors) = ready(&mut s);
        assert!(!errors.is_empty());
        assert_eq!(
            state.unwrap().doc.find("OK").unwrap().property.text,
            "before error"
        );
        s.stop();
        s.join();
    }

    #[test]
    fn rejects_legacy_and_filesystem_imports() {
        let legacy = crate::document::parse(include_str!("../ADF/CIAO.XML")).unwrap();
        assert!(Session::start(legacy, || {}).is_err());
        let mut s = Session::start(project("import \"secret.rhai\";"), || {}).unwrap();
        assert!(!ready(&mut s).1.is_empty());
        s.stop();
        s.join();
    }

    #[test]
    fn ciao_example_produces_the_original_result_after_confirmation() {
        let doc = crate::document::parse(include_str!("../examples/ciao.xml")).unwrap();
        let property = doc.find("OK").unwrap().property.clone();
        let mut s = Session::start(doc, || {}).unwrap();
        assert!(ready(&mut s).1.is_empty());
        s.event("OK", false, property).unwrap();
        let dialog = loop {
            if let Update::Dialog(d) = s.updates.recv_timeout(Duration::from_secs(5)).unwrap() {
                break d;
            }
        };
        assert_eq!((&*dialog.title, &*dialog.text), ("DVD", "CIAO"));
        dialog.reply.send("ok".into()).unwrap();
        let (snapshot, errors) = ready(&mut s);
        assert!(errors.is_empty());
        let doc = snapshot.unwrap().doc;
        assert_eq!(doc.find("OK").unwrap().property.text, "SOMMA = 2");
        assert_eq!(doc.find("CIAO").unwrap().property.text, "ADF");
        s.stop();
        s.join();
    }

    #[test]
    fn change_sees_new_typed_value_and_programmatic_updates_do_not_recurse() {
        let mut doc = Document {
            module: "set_state(\"calls\", 0);".into(),
            ..Default::default()
        };
        let mut c = Control::new(Kind::Checkbox, "CHECK".into());
        c.events.change = "set_state(\"calls\", get_state(\"calls\") + 1); set_text(\"MAIN\", get_value(\"CHECK\").to_string()); set_value(\"CHECK\", false);".into();
        let mut property = c.property.clone();
        property.checked = true;
        doc.controls.items[0].controls.items.push(c);
        let mut s = Session::start(doc, || {}).unwrap();
        ready(&mut s);
        s.event("CHECK", true, property).unwrap();
        let (state, errors) = ready(&mut s);
        assert!(errors.is_empty());
        let state = state.unwrap();
        assert_eq!(state.doc.find("MAIN").unwrap().property.text, "true");
        assert!(!state.doc.find("CHECK").unwrap().property.checked);
        assert!(s.updates.try_recv().is_err());
        s.stop();
        s.join();
    }

    #[test]
    fn invalid_combobox_input_does_not_corrupt_the_model_or_stop_the_worker() {
        let mut doc = Document::default();
        let c = Control::new(Kind::Combobox, "CHOICE".into());
        let mut property = c.property.clone();
        doc.controls.items[0].controls.items.push(c);
        let mut s = Session::start(doc, || {}).unwrap();
        ready(&mut s);
        property.selected = 100;
        s.event("CHOICE", true, property.clone()).unwrap();
        assert!(!ready(&mut s).1.is_empty());
        property.selected = 1;
        s.event("CHOICE", true, property).unwrap();
        let (state, errors) = ready(&mut s);
        assert!(errors.is_empty());
        assert_eq!(
            state.unwrap().doc.find("CHOICE").unwrap().property.selected,
            1
        );
        s.stop();
        s.join();
    }

    #[test]
    fn disconnected_dialog_returns_error_instead_of_deadlocking() {
        let doc = project("message(\"Title\", \"Question\"); set_text(\"OK\", \"must not run\");");
        let property = doc.find("OK").unwrap().property.clone();
        let mut s = Session::start(doc, || {}).unwrap();
        ready(&mut s);
        s.event("OK", false, property).unwrap();
        loop {
            if let Update::Dialog(dialog) = s.updates.recv_timeout(Duration::from_secs(5)).unwrap()
            {
                drop(dialog);
                break;
            }
        }
        let (state, errors) = ready(&mut s);
        assert!(!errors.is_empty());
        assert_eq!(state.unwrap().doc.find("OK").unwrap().property.text, "OK");
        s.stop();
        s.join();
    }

    #[test]
    fn dialog_does_not_reset_the_operation_budget() {
        let doc = project(
            "for i in 0..15000 { let n = i + 1; } message(\"Budget\", \"Continue\"); for i in 0..15000 { let n = i + 1; } set_text(\"OK\", \"must not run\");",
        );
        let property = doc.find("OK").unwrap().property.clone();
        let mut s = Session::start(doc, || {}).unwrap();
        ready(&mut s);
        s.event("OK", false, property).unwrap();
        let dialog = loop {
            match s.updates.recv_timeout(Duration::from_secs(5)).unwrap() {
                Update::Dialog(d) => break d,
                Update::State(_) => {}
                Update::Error(error) => panic!("Budget exhausted before the dialog: {error}"),
                _ => panic!("Unexpected update"),
            }
        };
        dialog.reply.send("ok".into()).unwrap();
        let (state, errors) = ready(&mut s);
        assert!(!errors.is_empty());
        assert_eq!(state.unwrap().doc.find("OK").unwrap().property.text, "OK");
        s.stop();
        s.join();
    }

    #[test]
    fn recursion_array_size_and_dynamic_eval_are_bounded() {
        let oversized = format!("let data = [{}];", vec!["0"; 10_001].join(","));
        for script in [
            "fn recurse() { recurse(); } recurse();",
            &oversized,
            "eval(\"40+2\");",
            "set_value(\"OK\", true);",
        ] {
            let doc = project(script);
            let property = doc.find("OK").unwrap().property.clone();
            let mut s = Session::start(doc, || {}).unwrap();
            let (_, mut errors) = ready(&mut s);
            if errors.is_empty() {
                s.event("OK", false, property).unwrap();
                errors = ready(&mut s).1;
            }
            assert!(!errors.is_empty(), "{script}");
            s.stop();
            s.join();
        }
    }
}
