use super::{hidpp, DeviceKey, Session};
use hidapi::{BusType, HidApi};
use std::{
    collections::BTreeSet,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, SyncSender, TrySendError},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

pub(super) enum Discovered {
    Connected(DeviceKey, Session),
    Status {
        name: Option<String>,
        error: Option<String>,
    },
}

pub(super) struct Discovery {
    pub receiver: Receiver<Discovered>,
    pub connected: Arc<Mutex<BTreeSet<DeviceKey>>>,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl Discovery {
    pub fn new(stop: Arc<AtomicBool>) -> Result<Self, String> {
        let (sender, receiver) = mpsc::sync_channel(8);
        let connected = Arc::new(Mutex::new(BTreeSet::new()));
        let worker_stop = stop.clone();
        let worker_connected = connected.clone();
        let worker = thread::Builder::new()
            .name("mtool-logitech-discovery".into())
            .spawn(move || discover(worker_stop, worker_connected, sender))
            .map_err(|error| format!("罗技设备扫描启动失败：{error}"))?;
        Ok(Self {
            receiver,
            connected,
            stop,
            worker: Some(worker),
        })
    }
}

impl Drop for Discovery {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn discover(
    stop: Arc<AtomicBool>,
    connected: Arc<Mutex<BTreeSet<DeviceKey>>>,
    sender: SyncSender<Discovered>,
) {
    let mut api = None;
    let mut next_discovery = Instant::now();
    while !stop.load(Ordering::Relaxed) {
        if Instant::now() < next_discovery {
            thread::sleep(Duration::from_millis(50));
            continue;
        }
        next_discovery = Instant::now() + Duration::from_secs(5);
        let result = if let Some(api) = api.as_mut() {
            HidApi::refresh_devices(api)
        } else {
            HidApi::new().map(|created| api = Some(created))
        };
        if let Err(error) = result {
            let _ = sender.try_send(Discovered::Status {
                name: None,
                error: Some(format!("无法读取罗技设备：{error}")),
            });
            continue;
        }
        let Some(api) = api.as_ref() else { continue };
        let mut discovery_error = None;
        let mut discovered_name = None;
        let mut devices: Vec<_> = api
            .device_list()
            .filter(|device| device.vendor_id() == 0x046d)
            .take(16)
            .collect();
        devices.sort_by_key(|device| !matches!(device.bus_type(), BusType::Bluetooth));
        for device in devices {
            if stop.load(Ordering::Relaxed) {
                break;
            }
            let bluetooth = matches!(device.bus_type(), BusType::Bluetooth)
                && device.usage_page() == 1
                && device.usage() == 2;
            let receiver = matches!(device.bus_type(), BusType::Usb)
                && device.usage_page() >= 0xff00
                && device.usage() == 1;
            if !bluetooth && !receiver {
                continue;
            }
            if bluetooth {
                discovered_name = device.product_string().map(str::to_owned);
            }
            if !unsafe { super::super::CGPreflightListenEventAccess() } {
                discovery_error =
                    Some("罗技额外按键需要输入监控权限，授权后请重新启动 MTool".into());
                continue;
            }
            let indices: &[u8] = if bluetooth {
                &[0xff]
            } else {
                &[1, 2, 3, 4, 5, 6]
            };
            for index in indices {
                if stop.load(Ordering::Relaxed) {
                    break;
                }
                let key = (device.path().to_owned(), *index);
                let skip = match connected.lock() {
                    Ok(connected) => connected.contains(&key) || connected.len() >= 8,
                    Err(_) => {
                        let _ = sender.try_send(Discovered::Status {
                            name: discovered_name,
                            error: Some("罗技连接状态不可用，请重启 MTool".into()),
                        });
                        return;
                    }
                };
                if skip {
                    continue;
                }
                let result = device
                    .open_device(api)
                    .map_err(|error| {
                        format!("无法连接罗技设备，请在系统设置中允许 MTool「输入监控」：{error}")
                    })
                    .and_then(|device| hidpp::Connection::discover(device, *index));
                match result {
                    Ok(connection) => {
                        let session = Session {
                            connection,
                            name: device
                                .product_string()
                                .unwrap_or("Logitech Mouse")
                                .to_owned(),
                        };
                        if let Ok(mut connected) = connected.lock() {
                            connected.insert(key.clone());
                        }
                        match sender.try_send(Discovered::Connected(key.clone(), session)) {
                            Ok(()) => {}
                            Err(error) => {
                                if let Ok(mut connected) = connected.lock() {
                                    connected.remove(&key);
                                }
                                if matches!(error, TrySendError::Disconnected(_)) {
                                    return;
                                }
                            }
                        }
                    }
                    Err(error) if bluetooth => discovery_error = Some(error),
                    Err(_) => {}
                }
            }
        }
        if matches!(
            sender.try_send(Discovered::Status {
                name: discovered_name,
                error: discovery_error
            }),
            Err(TrySendError::Disconnected(_))
        ) {
            return;
        }
    }
}
