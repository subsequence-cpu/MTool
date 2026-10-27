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
    dpi_feature: u8,
    pub dpi: Vec<SensorDpi>,
    pub dpi_error: Option<String>,
    dpi_changed: bool,
    input_pending: bool,
    controls: Vec<(u16, u16)>,
    diverted: BTreeSet<u16>,
    active: BTreeSet<u16>,
    pending: VecDeque<Vec<u8>>,
}

pub(super) struct SensorDpi {
    pub sensor: u8,
    pub current: u16,
    pub values: Vec<u16>,
}

impl Connection {
    pub fn discover(device: HidDevice, index: u8) -> Result<Self, String> {
        let mut connection = Self {
            device,
            software_id: 0x0d,
            index,
            feature: 0,
            dpi_feature: 0,
            dpi: Vec::new(),
            dpi_error: None,
            dpi_changed: false,
            input_pending: true,
            controls: Vec::new(),
            diverted: BTreeSet::new(),
            active: BTreeSet::new(),
            pending: VecDeque::new(),
        };
        let root = connection.request(0, 0, &[0x1b, 0x04, 0])?;
        connection.feature = root[0];
        let count = if connection.feature != 0 {
            connection.request(connection.feature, 0, &[])?[0].min(32)
        } else {
            0
        };
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
        let dpi = connection
            .request(0, 0, &[0x22, 0x01, 0])
            .and_then(|reply| {
                connection.dpi_feature = reply[0];
                if connection.dpi_feature != 0 {
                    connection.discover_dpi()?;
                }
                Ok(())
            });
        if let Err(reason) = dpi {
            connection.dpi.clear();
            connection.dpi_error = Some(reason);
        }
        if connection.controls.is_empty() && connection.dpi.is_empty() {
            return Err("未发现支持的罗技按键或硬件 DPI 功能".into());
        }
        Ok(connection)
    }

    pub fn buttons(&self) -> Vec<u16> {
        self.controls.iter().map(|(_, button)| *button).collect()
    }

    pub fn identity(&mut self) -> Option<String> {
        let feature = self.request(0, 0, &[0x00, 0x03, 0]).ok()?[0];
        if feature == 0 {
            return None;
        }
        let reply = self.request(feature, 0, &[]).ok()?;
        let unit = reply.get(1..5)?;
        if unit.iter().all(|byte| *byte == 0) || unit.iter().all(|byte| *byte == 0xff) {
            return None;
        }
        Some(format!(
            "logitech-{}",
            unit.iter()
                .chain(reply.get(7..13)?)
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        ))
    }

    pub fn take_input_pending(&mut self) -> bool {
        std::mem::take(&mut self.input_pending)
    }

    fn discover_dpi(&mut self) -> Result<(), String> {
        let count = self.request(self.dpi_feature, 0, &[])?[0];
        if !(1..=8).contains(&count) {
            return Err("罗技 DPI 传感器数量无效".into());
        }
        for sensor in 0..count {
            let list = self.request(self.dpi_feature, 1, &[sensor])?;
            if list.first() != Some(&sensor) {
                return Err("罗技 DPI 档位响应无效".into());
            }
            let values = dpi_values(&list[1..])?;
            let current = self.read_dpi(sensor)?;
            if !values.contains(&current) {
                return Err("罗技当前 DPI 不在设备支持的档位中".into());
            }
            self.dpi.push(SensorDpi {
                sensor,
                current,
                values,
            });
        }
        Ok(())
    }

    fn read_dpi(&mut self, sensor: u8) -> Result<u16, String> {
        let reply = self.request(self.dpi_feature, 2, &[sensor])?;
        if reply.len() < 5 || reply[0] != sensor {
            return Err("罗技 DPI 响应无效".into());
        }
        let current = u16::from_be_bytes([reply[1], reply[2]]);
        let current = if current == 0 {
            u16::from_be_bytes([reply[3], reply[4]])
        } else {
            current
        };
        if current == 0 {
            return Err("无法读取罗技当前 DPI".into());
        }
        Ok(current)
    }

    pub fn take_dpi_changed(&mut self) -> bool {
        std::mem::take(&mut self.dpi_changed)
    }

    fn dpi_notification(&mut self, report: &[u8]) {
        if self.dpi_feature == 0
            || report.len() < 7
            || !matches!(report[0], 0x10 | 0x11)
            || report[1..4] != [self.index, self.dpi_feature, 0]
        {
            return;
        }
        let value = u16::from_be_bytes([report[5], report[6]]);
        if let Some(dpi) = self
            .dpi
            .iter_mut()
            .find(|dpi| dpi.sensor == report[4] && dpi.values.contains(&value))
        {
            self.dpi_changed |= dpi.current != value;
            dpi.current = value;
        }
    }

    pub fn set_dpi(&mut self, sensor: u8, value: u16) -> Result<(), String> {
        let Some(index) = self.dpi.iter().position(|dpi| dpi.sensor == sensor) else {
            return Err("未找到罗技 DPI 传感器".into());
        };
        if !self.dpi[index].values.contains(&value) {
            return Err("DPI 不在设备支持的档位中".into());
        }
        let [high, low] = value.to_be_bytes();
        self.request(self.dpi_feature, 3, &[sensor, high, low])?;
        let current = self.read_dpi(sensor)?;
        self.dpi[index].current = current;
        if current != value {
            return Err("鼠标未应用该 DPI，请检查板载配置或其他鼠标软件".into());
        }
        Ok(())
    }

    pub fn keep_dpi(&mut self, sensor: u8, value: u16) -> Result<(), String> {
        let Some(index) = self.dpi.iter().position(|dpi| dpi.sensor == sensor) else {
            return Err("未找到罗技 DPI 传感器".into());
        };
        if !self.dpi[index].values.contains(&value) {
            return Err("保存的 DPI 不在设备支持的档位中".into());
        }
        let previous = self.dpi[index].current;
        let current = self.read_dpi(sensor)?;
        self.dpi[index].current = current;
        let result = if current == value {
            Ok(())
        } else {
            self.set_dpi(sensor, value)
        };
        self.dpi_changed |= previous != self.dpi[index].current;
        result
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
            self.dpi_notification(report);
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
            self.input_pending = true;
            self.dpi_notification(report);
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

fn dpi_values(bytes: &[u8]) -> Result<Vec<u16>, String> {
    let mut values = BTreeSet::new();
    let mut words = bytes.as_chunks::<2>().0.iter();
    let mut previous = 0;
    while let Some(bytes) = words.next() {
        let value = u16::from_be_bytes([bytes[0], bytes[1]]);
        if value == 0 {
            break;
        }
        if value & 0xe000 == 0xe000 {
            let step = value & 0x1fff;
            let end = words.next().ok_or("罗技 DPI 范围不完整")?;
            let end = u16::from_be_bytes([end[0], end[1]]);
            if previous == 0 || step == 0 || end <= previous || end >= 0xe000 {
                return Err("罗技 DPI 范围无效".into());
            }
            for value in ((u32::from(previous) + u32::from(step))..=u32::from(end))
                .step_by(usize::from(step))
            {
                values.insert(value as u16);
                if values.len() > 4096 {
                    return Err("罗技 DPI 档位过多".into());
                }
            }
            previous = end;
        } else {
            values.insert(value);
            previous = value;
        }
    }
    if values.len() < 2 {
        return Err("罗技 DPI 档位列表无效".into());
    }
    Ok(values.into_iter().collect())
}

impl Drop for Connection {
    fn drop(&mut self) {
        if let Err(error) = self.restore() {
            eprintln!("Could not restore Logitech button reporting: {error}");
        }
    }
}
