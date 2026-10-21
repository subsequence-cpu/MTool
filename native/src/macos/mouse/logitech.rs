use super::{EventSender, Message, Runtime};
use mtool_core::{MouseAction, MouseSettings};
use std::{
    collections::BTreeMap,
    ffi::CString,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

mod discovery;
#[path = "hidpp.rs"]
mod hidpp;
use discovery::{Discovered, Discovery};

pub(super) struct Service {
    settings: Arc<Mutex<MouseSettings>>,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl Service {
    pub fn new(settings: MouseSettings, runtime: Arc<Mutex<Runtime>>, sender: EventSender) -> Self {
        let settings = Arc::new(Mutex::new(settings));
        let stop = Arc::new(AtomicBool::new(false));
        let worker_settings = settings.clone();
        let worker_stop = stop.clone();
        let worker_runtime = runtime.clone();
        let worker = thread::Builder::new()
            .name("mtool-logitech".into())
            .spawn(move || {
                if let Err(error) =
                    run(worker_settings, worker_stop, worker_runtime.clone(), sender)
                {
                    if let Ok(mut state) = worker_runtime.lock() {
                        state.logitech_error = Some(error);
                    }
                }
            });
        let worker = match worker {
            Ok(worker) => Some(worker),
            Err(error) => {
                if let Ok(mut state) = runtime.lock() {
                    state.logitech_error = Some(format!("罗技服务启动失败：{error}"));
                }
                None
            }
        };
        Self {
            settings,
            stop,
            worker,
        }
    }

    pub fn configure(&self, settings: MouseSettings) {
        if let Ok(mut current) = self.settings.lock() {
            *current = settings;
        }
    }
}

impl Drop for Service {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

type DeviceKey = (CString, u8);

struct Session {
    connection: hidpp::Connection,
    name: String,
}

fn run(
    settings: Arc<Mutex<MouseSettings>>,
    stop: Arc<AtomicBool>,
    runtime: Arc<Mutex<Runtime>>,
    sender: EventSender,
) -> Result<(), String> {
    let discovery = Discovery::new(stop.clone())?;
    let mut sessions: BTreeMap<DeviceKey, Session> = BTreeMap::new();
    let mut next_configure = Instant::now();
    let mut discovered_name = None;
    let mut discovery_error = None;
    let mut publish_devices = false;
    while !stop.load(Ordering::Relaxed) {
        for message in discovery.receiver.try_iter().take(8) {
            publish_devices = true;
            match message {
                Discovered::Connected(key, session) => {
                    sessions.insert(key, session);
                    next_configure = Instant::now();
                }
                Discovered::Status { name, error } => {
                    discovered_name = name;
                    discovery_error = error;
                }
            }
        }
        let configure = Instant::now() >= next_configure;
        let current = settings
            .lock()
            .map_err(|_| "罗技设置暂时不可用".to_string())?
            .clone();
        let (capturing, running) = runtime
            .lock()
            .map(|state| (state.capturing(), state.running))
            .unwrap_or_default();
        let mut error = None;
        let mut disconnected = Vec::new();
        for (key, session) in &mut sessions {
            if configure {
                if let Err(reason) = session.connection.configure(|button| {
                    capturing
                        || (running
                            && current.enabled
                            && current.button_action(button) != MouseAction::Default)
                }) {
                    error = Some(reason);
                    disconnected.push(key.clone());
                    continue;
                }
            }
            match session.connection.poll() {
                Ok(events) => {
                    for (button, down) in events {
                        if sender
                            .send(Message::LogitechButton { button, down })
                            .is_err()
                        {
                            return Ok(());
                        }
                    }
                }
                Err(reason) => {
                    error = Some(reason);
                    disconnected.push(key.clone());
                }
            }
        }
        if !disconnected.is_empty() {
            let _ = sender.send(Message::LogitechDisconnected);
        }
        for key in disconnected {
            sessions.remove(&key);
            if let Ok(mut connected) = discovery.connected.lock() {
                connected.remove(&key);
            }
            publish_devices = true;
        }
        if publish_devices {
            if let Ok(mut state) = runtime.lock() {
                state.logitech_device = sessions
                    .values()
                    .next()
                    .map(|session| session.name.clone())
                    .or_else(|| discovered_name.clone());
                state.logitech_buttons = sessions
                    .values()
                    .flat_map(|session| session.connection.buttons())
                    .collect();
                state.logitech_buttons.sort_unstable();
                state.logitech_buttons.dedup();
                if sessions.is_empty() && error.is_none() {
                    state.logitech_error = discovery_error.clone();
                }
            }
            publish_devices = false;
        }
        if error.is_some() || (configure && !sessions.is_empty()) {
            if let Ok(mut state) = runtime.lock() {
                state.logitech_error = error;
            }
        }
        if configure {
            next_configure = Instant::now() + Duration::from_millis(250);
        }
        thread::sleep(Duration::from_millis(8));
    }
    drop(sessions);
    Ok(())
}
