use super::{EventSender, Message, Runtime};
use mtool_core::{MouseAction, MouseDpi, MouseSettings};
use std::{
    collections::BTreeMap,
    ffi::CString,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{Receiver, SyncSender},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

mod discovery;
#[path = "hidpp.rs"]
mod hidpp;
use discovery::{Discovered, Discovery};

pub(super) struct DpiRequest {
    pub id: String,
    pub value: u16,
    pub deadline: Instant,
    pub response: SyncSender<Result<MouseDpi, String>>,
}

pub(super) struct Service {
    settings: Arc<Mutex<MouseSettings>>,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl Service {
    pub fn new(
        settings: MouseSettings,
        runtime: Arc<Mutex<Runtime>>,
        sender: EventSender,
        dpi_receiver: Receiver<DpiRequest>,
        dpi: BTreeMap<String, u16>,
    ) -> Self {
        let settings = Arc::new(Mutex::new(settings));
        let stop = Arc::new(AtomicBool::new(false));
        let worker_settings = settings.clone();
        let worker_stop = stop.clone();
        let worker_runtime = runtime.clone();
        let worker = thread::Builder::new()
            .name("mtool-logitech".into())
            .spawn(move || {
                if let Err(error) = run(
                    worker_settings,
                    worker_stop,
                    worker_runtime.clone(),
                    sender,
                    dpi_receiver,
                    dpi,
                ) {
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
    identity: String,
    next_dpi_check: Instant,
}

fn dpi_id(session: &Session, sensor: u8) -> String {
    format!("{}-{sensor}", session.identity)
}

fn dpi_status(session: &Session, dpi: &hidpp::SensorDpi) -> MouseDpi {
    MouseDpi {
        id: dpi_id(session, dpi.sensor),
        name: if session.connection.dpi.len() > 1 {
            format!("{} · 传感器 {}", session.name, dpi.sensor + 1)
        } else {
            session.name.clone()
        },
        current: dpi.current,
        values: dpi.values.clone(),
    }
}

fn dpi_devices(sessions: &BTreeMap<DeviceKey, Session>) -> Vec<MouseDpi> {
    sessions
        .values()
        .flat_map(|session| {
            session
                .connection
                .dpi
                .iter()
                .map(|dpi| dpi_status(session, dpi))
        })
        .collect()
}

fn apply_dpi(
    sessions: &mut BTreeMap<DeviceKey, Session>,
    request: &DpiRequest,
) -> Result<MouseDpi, String> {
    if Instant::now() >= request.deadline {
        return Err("DPI 设置请求已超时，请重试".into());
    }
    for session in sessions.values_mut() {
        if let Some(index) = session
            .connection
            .dpi
            .iter()
            .position(|dpi| dpi_id(session, dpi.sensor) == request.id)
        {
            let sensor = session.connection.dpi[index].sensor;
            session.connection.set_dpi(sensor, request.value)?;
            session.next_dpi_check = Instant::now() + Duration::from_secs(2);
            return Ok(dpi_status(session, &session.connection.dpi[index]));
        }
    }
    Err("鼠标已断开或不支持硬件 DPI 调节".into())
}

fn run(
    settings: Arc<Mutex<MouseSettings>>,
    stop: Arc<AtomicBool>,
    runtime: Arc<Mutex<Runtime>>,
    sender: EventSender,
    dpi_receiver: Receiver<DpiRequest>,
    mut dpi: BTreeMap<String, u16>,
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
                    sessions.insert(key, *session);
                    next_configure = Instant::now();
                }
                Discovered::Status { name, error } => {
                    discovered_name = name;
                    discovery_error = error;
                }
            }
        }
        if let Ok(request) = dpi_receiver.try_recv() {
            let result = apply_dpi(&mut sessions, &request);
            if let Ok(device) = &result {
                dpi.insert(device.id.clone(), device.current);
            }
            if let Ok(mut state) = runtime.lock() {
                state.logitech_dpi = dpi_devices(&sessions);
            }
            let _ = request.response.try_send(result);
            publish_devices = true;
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
            if Instant::now() >= session.next_dpi_check && session.connection.take_input_pending() {
                session.next_dpi_check = Instant::now() + Duration::from_secs(2);
                let wanted: Vec<_> = session
                    .connection
                    .dpi
                    .iter()
                    .filter_map(|sensor| {
                        dpi.get(&dpi_id(session, sensor.sensor))
                            .map(|value| (sensor.sensor, *value))
                    })
                    .collect();
                if !wanted.is_empty() {
                    let result = wanted
                        .into_iter()
                        .try_for_each(|(sensor, value)| session.connection.keep_dpi(sensor, value));
                    session.connection.dpi_error = result.err();
                }
                if session.connection.dpi_error.is_some() {
                    session.next_dpi_check = Instant::now() + Duration::from_secs(5);
                }
            }
            publish_devices |= session.connection.take_dpi_changed();
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
                state.logitech_dpi = dpi_devices(&sessions);
                if sessions.is_empty() && error.is_none() {
                    state.logitech_error = discovery_error.clone();
                }
            }
            publish_devices = false;
        }
        if error.is_some() || (configure && !sessions.is_empty()) {
            if let Ok(mut state) = runtime.lock() {
                state.logitech_error = error.or_else(|| {
                    sessions
                        .values()
                        .find_map(|session| session.connection.dpi_error.clone())
                });
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
