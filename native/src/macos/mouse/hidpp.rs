use hidapi::HidDevice;
use std::{
    collections::{BTreeSet, VecDeque},
    time::{Duration, Instant},
};

fn logical_button(cid: u16) -> Option<u16> {
    match cid {
        0x00c3 => Some(6),
        0x00c4 => Some(7),
        0x00d7 => Some(8),
        _ => None,
    }
}

fn packet(index: u8, feature: u8, function: u8, software_id: u8, params: &[u8]) -> [u8; 20] {
    let mut report = [0; 20];
    report[..4].copy_from_slice(&[0x11, index, feature, function << 4 | software_id]);
    report[4..4 + params.len()].copy_from_slice(params);
    report
}

fn response_matches(report: &[u8], request: &[u8]) -> bool {
    report.len() >= 7 && matches!(report[0], 0x10 | 0x11) && report[1..4] == request[1..4]
}

fn button_state(report: &[u8], index: u8, feature: u8) -> Option<BTreeSet<u16>> {
    if report.len() < 12 || report[..4] != [0x11, index, feature, 0] {
        return None;
    }
    Some(
        report[4..12]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|bytes| u16::from_be_bytes([bytes[0], bytes[1]]))
            .filter(|cid| *cid != 0)
            .collect(),
    )
}

pub(super) struct Connection {
    device: HidDevice,
    software_id: u8,
    index: u8,
    feature: u8,
    controls: Vec<(u16, u16)>,
    diverted: BTreeSet<u16>,
    active: BTreeSet<u16>,
    pending: VecDeque<Vec<u8>>,
}

impl Connection {
    pub fn discover(device: HidDevice, index: u8) -> Result<Self, String> {
        let mut connection = Self {
            device,
            software_id: 0x0d,
            index,
            feature: 0,
            controls: Vec::new(),
            diverted: BTreeSet::new(),
            active: BTreeSet::new(),
            pending: VecDeque::new(),
        };
        let root = connection.request(0, 0, &[0x1b, 0x04, 0])?;
        connection.feature = root[0];
        if connection.feature == 0 {
            return Err("设备不支持 HID++ 可编程按键".into());
        }
        let count = connection.request(connection.feature, 0, &[])?[0].min(32);
        let discovery_deadline = Instant::now() + Duration::from_secs(2);
        for offset in 0..count {
            if Instant::now() > discovery_deadline {
                return Err("罗技按键能力查询超时，请重试".into());
            }
            let info = connection.request(connection.feature, 1, &[offset])?;
            if info.len() < 5 || info[4] & 0x20 == 0 {
                continue;
            }
            let cid = u16::from_be_bytes([info[0], info[1]]);
            if let Some(button) = logical_button(cid) {
                connection.controls.push((cid, button));
            }
        }
        if connection.controls.is_empty() {
            return Err("未发现可接入的罗技额外按键".into());
        }
        Ok(connection)
    }

    pub fn buttons(&self) -> Vec<u16> {
        self.controls.iter().map(|(_, button)| *button).collect()
    }

    fn request(&mut self, feature: u8, function: u8, params: &[u8]) -> Result<Vec<u8>, String> {
        let request = packet(self.index, feature, function, self.software_id, params);
        self.software_id = self.software_id % 15 + 1;
        let written = self
            .device
            .write(&request)
            .map_err(|error| format!("HID++ 发送失败：{error}"))?;
        if written != request.len() {
            return Err("HID++ 报告发送不完整".into());
        }
        let deadline = Instant::now() + Duration::from_millis(250);
        let mut buffer = [0; 64];
        while Instant::now() < deadline {
            let length = self
                .device
                .read_timeout(&mut buffer, 10)
                .map_err(|error| format!("HID++ 读取失败：{error}"))?;
            let report = &buffer[..length];
            if response_matches(report, &request) {
                return Ok(report[4..].to_vec());
            }
            if report.len() >= 7
                && report[1] == self.index
                && report[2] == 0xff
                && report[3..5] == request[2..4]
            {
                return Err(format!("鼠标拒绝 HID++ 请求（错误 {}）", report[5]));
            }
            if button_state(report, self.index, self.feature).is_some() {
                if self.pending.len() >= 64 {
                    return Err("HID++ 事件过多，请重试".into());
                }
                self.pending.push_back(report.to_vec());
            }
        }
        Err("鼠标未响应 HID++ 请求，请确认连接并退出接管按键的其他鼠标软件".into())
    }

    fn set_diversion(&mut self, cid: u16, enabled: bool) -> Result<(), String> {
        let [high, low] = cid.to_be_bytes();
        self.request(
            self.feature,
            3,
            &[high, low, if enabled { 3 } else { 2 }, 0, 0],
        )?;
        Ok(())
    }

    pub fn configure(&mut self, mut wanted: impl FnMut(u16) -> bool) -> Result<(), String> {
        for (cid, button) in self.controls.clone() {
            let enabled = wanted(button);
            if enabled == self.diverted.contains(&cid) || self.active.contains(&cid) {
                continue;
            }
            if enabled {
                let current = self.request(self.feature, 2, &cid.to_be_bytes())?;
                if current.len() < 3 || current[2] & 1 != 0 {
                    return Err(format!(
                        "按键 {button} 正被其他鼠标软件接管，请退出对应软件后重试"
                    ));
                }
                self.diverted.insert(cid);
                self.set_diversion(cid, true)?;
                let current = self.request(self.feature, 2, &cid.to_be_bytes())?;
                if current.len() < 3 || current[2] & 1 == 0 {
                    return Err(format!("按键 {button} 未能启用，请重试"));
                }
            } else {
                self.set_diversion(cid, false)?;
                self.diverted.remove(&cid);
            }
        }
        Ok(())
    }

    pub fn poll(&mut self) -> Result<Vec<(u16, bool)>, String> {
        let mut changes = Vec::new();
        for _ in 0..64 {
            let mut buffer = [0; 64];
            let queued = self.pending.pop_front();
            let report = if let Some(report) = queued.as_ref() {
                report.as_slice()
            } else {
                let length = self
                    .device
                    .read_timeout(&mut buffer, 0)
                    .map_err(|error| format!("罗技设备连接已断开：{error}"))?;
                if length == 0 {
                    break;
                }
                &buffer[..length]
            };
            if let Some(next) = button_state(report, self.index, self.feature) {
                for cid in self.active.difference(&next) {
                    if self.diverted.contains(cid) {
                        if let Some(button) = logical_button(*cid) {
                            changes.push((button, false));
                        }
                    }
                }
                for cid in next.difference(&self.active) {
                    if self.diverted.contains(cid) {
                        if let Some(button) = logical_button(*cid) {
                            changes.push((button, true));
                        }
                    }
                }
                self.active = next;
            }
        }
        Ok(changes)
    }

    fn restore(&mut self) -> Result<(), String> {
        let mut failure = None;
        for cid in self.diverted.clone() {
            match self.set_diversion(cid, false) {
                Ok(()) => {
                    self.diverted.remove(&cid);
                }
                Err(error) => failure = Some(error),
            }
        }
        failure.map_or(Ok(()), Err)
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        if let Err(error) = self.restore() {
            eprintln!("Could not restore Logitech button reporting: {error}");
        }
    }
}
