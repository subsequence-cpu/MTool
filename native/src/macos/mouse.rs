use crate::common::mouse::{
    gesture::{Direction, Gesture},
    motion::Motion,
};
use core_foundation::{
    base::TCFType,
    mach_port::{CFMachPort, CFMachPortInvalidate, CFMachPortRef},
    runloop::{kCFRunLoopDefaultMode, CFRunLoop, CFRunLoopSource},
};
use core_graphics::{
    event::{CGEvent, CGEventFlags, CGEventTapLocation, EventField, KeyCode, ScrollEventUnit},
    event_source::{CGEventSource, CGEventSourceStateID},
};
use mtool_core::{MouseAction, MouseDpi, MouseSettings, MouseStatus};
use std::{
    ffi::c_void,
    panic::{catch_unwind, AssertUnwindSafe},
    ptr,
    sync::{mpsc, Arc, Mutex, OnceLock},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

mod logitech;

const SCROLL: u32 = 22;
const OTHER_DOWN: u32 = 25;
const OTHER_UP: u32 = 26;
const OTHER_DRAGGED: u32 = 27;
const DISABLED_TIMEOUT: u32 = 0xffff_fffe;
const DISABLED_INPUT: u32 = 0xffff_ffff;
const EVENT_MARKER: i64 = 0x4d54_4f4f_4c4d;
const SCROLL_PHASE: u32 = 99;
const MOMENTUM_PHASE: u32 = 123;
const PERMIT_ALL_LOCAL_EVENTS: u32 = 7;
type EventRef = *mut c_void;
type TapCallback = unsafe extern "C" fn(*mut c_void, u32, EventRef, *mut c_void) -> EventRef;

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGEventTapCreate(
        tap: u32,
        place: u32,
        options: u32,
        mask: u64,
        callback: unsafe extern "C" fn(*mut c_void, u32, EventRef, *mut c_void) -> EventRef,
        info: *mut c_void,
    ) -> CFMachPortRef;
    fn CGEventTapEnable(tap: CFMachPortRef, enable: bool);
    fn CGEventTapIsEnabled(tap: CFMachPortRef) -> bool;
    fn CGEventGetIntegerValueField(event: EventRef, field: u32) -> i64;
    fn CGEventGetDoubleValueField(event: EventRef, field: u32) -> f64;
    fn CGEventSetIntegerValueField(event: EventRef, field: u32, value: i64);
    fn CGEventSetDoubleValueField(event: EventRef, field: u32, value: f64);
    fn CGEventGetFlags(event: EventRef) -> CGEventFlags;
    fn CGEventSourceSetLocalEventsSuppressionInterval(source: *const c_void, seconds: f64);
    fn CGEventSourceSetLocalEventsFilterDuringSuppressionState(
        source: *const c_void,
        filter: u32,
        state: u32,
    );
    fn CGPreflightListenEventAccess() -> bool;
    fn CGRequestListenEventAccess() -> bool;
}

pub fn open_input_monitoring_settings() -> mtool_core::Result<()> {
    use objc2_app_kit::NSWorkspace;
    use objc2_foundation::{NSString, NSURL};
    unsafe {
        CGRequestListenEventAccess();
    }
    let url = NSURL::URLWithString(&NSString::from_str(
        "x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent",
    ))
    .ok_or_else(|| mtool_core::AppError::new("settings_unavailable", "无法打开输入监控设置"))?;
    if !NSWorkspace::sharedWorkspace().openURL(&url) {
        return Err(mtool_core::AppError::new(
            "settings_unavailable",
            "无法打开输入监控设置",
        ));
    }
    Ok(())
}

#[derive(Default)]
struct Runtime {
    running: bool,
    error: Option<String>,
    capture_until: Option<Instant>,
    last_button: Option<u16>,
    button_sequence: u32,
    logitech_device: Option<String>,
    logitech_buttons: Vec<u16>,
    logitech_dpi: Vec<MouseDpi>,
    logitech_error: Option<String>,
}

impl Runtime {
    fn capturing(&self) -> bool {
        self.capture_until
            .is_some_and(|until| until > Instant::now())
    }

    fn status(&self, granted: bool) -> MouseStatus {
        MouseStatus {
            supported: true,
            accessibility_granted: granted,
            error: self.error.clone(),
            capturing: self.capturing(),
            last_button: self.last_button,
            button_sequence: self.button_sequence,
            logitech_device: self.logitech_device.clone(),
            logitech_buttons: self.logitech_buttons.clone(),
            logitech_dpi: self.logitech_dpi.clone(),
            logitech_error: self.logitech_error.clone(),
            input_monitoring_granted: unsafe { CGPreflightListenEventAccess() },
        }
    }
}

fn record_button(runtime: &Mutex<Runtime>, button: u16) -> bool {
    if let Ok(mut state) = runtime.lock() {
        if state.capturing() {
            state.last_button = Some(button);
            state.button_sequence = state.button_sequence.wrapping_add(1);
            state.capture_until = None;
            return true;
        }
    }
    false
}

struct WakeSource(CFRunLoopSource);
unsafe impl Send for WakeSource {}
unsafe impl Sync for WakeSource {}

#[derive(Clone)]
struct EventSender {
    channel: mpsc::Sender<Message>,
    source: Arc<WakeSource>,
    run_loop: Arc<OnceLock<CFRunLoop>>,
}

impl EventSender {
    fn send(&self, message: Message) -> Result<(), mpsc::SendError<Message>> {
        use core_foundation::runloop::{CFRunLoopSourceSignal, CFRunLoopWakeUp};
        self.channel.send(message)?;
        unsafe {
            CFRunLoopSourceSignal(self.source.0.as_concrete_TypeRef());
            if let Some(run_loop) = self.run_loop.get() {
                CFRunLoopWakeUp(run_loop.as_concrete_TypeRef());
            }
        }
        Ok(())
    }
}

extern "C" fn wake_perform(_info: *const c_void) {}

enum Message {
    Configure(MouseSettings),
    Wake,
    LogitechButton { button: u16, down: bool },
    LogitechDisconnected,
    Stop,
}

pub(crate) struct MouseService {
    sender: EventSender,
    runtime: Arc<Mutex<Runtime>>,
    worker: Mutex<Option<JoinHandle<()>>>,
    dpi_sender: mpsc::SyncSender<logitech::DpiRequest>,
}

impl MouseService {
    pub fn new(settings: MouseSettings) -> Self {
        use core_foundation::runloop::{CFRunLoopSourceContext, CFRunLoopSourceCreate};
        let (channel, receiver) = mpsc::channel();
        let (dpi_sender, dpi_receiver) = mpsc::sync_channel(4);
        let mut context = CFRunLoopSourceContext {
            version: 0,
            info: ptr::null_mut(),
            retain: None,
            release: None,
            copyDescription: None,
            equal: None,
            hash: None,
            schedule: None,
            cancel: None,
            perform: wake_perform,
        };
        let raw = unsafe { CFRunLoopSourceCreate(ptr::null(), 0, &mut context) };
        assert!(!raw.is_null(), "Unable to create mouse wake source");
        let sender = EventSender {
            channel,
            source: Arc::new(WakeSource(unsafe {
                CFRunLoopSource::wrap_under_create_rule(raw)
            })),
            run_loop: Arc::new(OnceLock::new()),
        };
        let runtime = Arc::new(Mutex::new(Runtime::default()));
        let worker_runtime = runtime.clone();
        let worker_sender = sender.clone();
        let worker = thread::Builder::new()
            .name("mtool-mouse".into())
            .spawn(move || {
                run(
                    settings,
                    receiver,
                    worker_sender,
                    worker_runtime,
                    dpi_receiver,
                )
            });
        let worker = match worker {
            Ok(worker) => Some(worker),
            Err(error) => {
                publish(&runtime, false, Some(format!("鼠标服务启动失败：{error}")));
                None
            }
        };
        Self {
            sender,
            runtime,
            worker: Mutex::new(worker),
            dpi_sender,
        }
    }

    pub fn configure(&self, settings: MouseSettings) {
        if self.sender.send(Message::Configure(settings)).is_err() {
            publish(
                &self.runtime,
                false,
                Some("鼠标服务已停止，请重新启动 MTool".into()),
            );
        }
    }

    pub fn set_dpi(&self, id: String, value: u16) -> mtool_core::Result<MouseDpi> {
        let (response, receiver) = mpsc::sync_channel(1);
        self.dpi_sender
            .try_send(logitech::DpiRequest {
                id,
                value,
                deadline: Instant::now() + Duration::from_secs(2),
                response,
            })
            .map_err(|_| {
                mtool_core::AppError::new("dpi_unavailable", "DPI 服务繁忙或已停止，请重试")
            })?;
        receiver
            .recv_timeout(Duration::from_secs(3))
            .map_err(|_| mtool_core::AppError::new("dpi_timeout", "DPI 设置超时，请检查鼠标连接"))?
            .map_err(|reason| mtool_core::AppError::new("dpi_failed", reason))
    }

    pub fn status(&self) -> MouseStatus {
        let granted = super::finder::accessibility_granted();
        match self.runtime.lock() {
            Ok(runtime) => runtime.status(granted),
            Err(_) => Runtime {
                error: Some("鼠标服务状态不可用，请重新启动 MTool".into()),
                ..Runtime::default()
            }
            .status(granted),
        }
    }

    pub fn capture(&self, active: bool) -> mtool_core::Result<()> {
        if active && !super::finder::accessibility_granted() {
            return Err(mtool_core::AppError::new(
                "accessibility_required",
                "请先授予 MTool 辅助功能权限",
            ));
        }
        let mut state = crate::common::state::lock(&self.runtime)?;
        state.capture_until = active.then(|| Instant::now() + Duration::from_secs(15));
        if active {
            state.last_button = None;
        }
        self.sender.send(Message::Wake).map_err(|_| {
            mtool_core::AppError::new("mouse_stopped", "鼠标服务已停止，请重新启动 MTool")
        })
    }

    pub fn shutdown(&self) {
        let _ = self.sender.send(Message::Stop);
        if let Ok(mut worker) = self.worker.lock() {
            if let Some(worker) = worker.take() {
                let _ = worker.join();
            }
        }
    }
}

impl Drop for MouseService {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn publish(runtime: &Mutex<Runtime>, running: bool, error: Option<String>) {
    if let Ok(mut state) = runtime.lock() {
        state.running = running;
        state.error = error;
    }
}

const BUTTON_EVENTS: &[u32] = &[1, 3, SCROLL, OTHER_DOWN, OTHER_UP, OTHER_DRAGGED];

struct Filter {
    port: CFMachPort,
    source: CFRunLoopSource,
}

impl Filter {
    fn new(
        events: &[u32],
        context: *mut c_void,
        enabled: bool,
        handler: TapCallback,
    ) -> Result<Self, String> {
        let mask = events
            .iter()
            .fold(0u64, |mask, event| mask | (1u64 << event));
        let raw = unsafe { CGEventTapCreate(1, 0, 0, mask, handler, context) };
        if raw.is_null() {
            return Err("无法监听鼠标事件，请确认辅助功能授权；授权后仍失败可重启 MTool".into());
        }
        let port = unsafe { CFMachPort::wrap_under_create_rule(raw) };
        let source = match port.create_runloop_source(0) {
            Ok(source) => source,
            Err(_) => {
                unsafe { CFMachPortInvalidate(raw) };
                return Err("无法创建鼠标事件循环".into());
            }
        };
        unsafe { CGEventTapEnable(raw, enabled) };
        CFRunLoop::get_current().add_source(&source, unsafe { kCFRunLoopDefaultMode });
        Ok(Self { port, source })
    }

    fn enabled(&self) -> bool {
        unsafe { CGEventTapIsEnabled(self.port.as_concrete_TypeRef()) }
    }
}

impl Drop for Filter {
    fn drop(&mut self) {
        unsafe {
            CGEventTapEnable(self.port.as_concrete_TypeRef(), false);
            CFMachPortInvalidate(self.port.as_concrete_TypeRef());
        }
        CFRunLoop::get_current().remove_source(&self.source, unsafe { kCFRunLoopDefaultMode });
    }
}

struct Tap {
    primary: Filter,
    _movement: Filter,
    processor: Box<Processor>,
}

impl Tap {
    fn new(settings: MouseSettings, runtime: Arc<Mutex<Runtime>>) -> Result<Self, String> {
        let event_source = CGEventSource::new(CGEventSourceStateID::Private)
            .map_err(|_| "无法创建鼠标事件源".to_string())?;
        let raw_source = ptr::from_ref(&*event_source).cast();
        unsafe {
            CGEventSourceSetLocalEventsSuppressionInterval(raw_source, 0.0);
            for state in 0..=1 {
                CGEventSourceSetLocalEventsFilterDuringSuppressionState(
                    raw_source,
                    PERMIT_ALL_LOCAL_EVENTS,
                    state,
                );
            }
        }
        let pointer_source = CGEventSource::new(CGEventSourceStateID::CombinedSessionState)
            .map_err(|_| "无法读取当前鼠标位置".to_string())?;
        let mut processor = Box::new(Processor {
            settings,
            motion: Motion::default(),
            source: event_source,
            pointer_source,
            port: ptr::null_mut(),
            movement_port: ptr::null_mut(),
            suppressed: [false; 30],
            held: None,
            runtime,
            flags: CGEventFlags::empty(),
            last_frame: Instant::now(),
            fault: None,
        });
        let context = (&mut *processor as *mut Processor).cast();
        let primary = Filter::new(BUTTON_EVENTS, context, true, callback)?;
        processor.port = primary.port.as_concrete_TypeRef();
        let movement = Filter::new(&[5, 6, 7], context, false, movement_callback)?;
        processor.movement_port = movement.port.as_concrete_TypeRef();
        Ok(Self {
            primary,
            _movement: movement,
            processor,
        })
    }

    fn enabled(&self) -> bool {
        self.primary.enabled()
    }
}

impl Drop for Tap {
    fn drop(&mut self) {
        self.processor.cancel_gesture();
    }
}

fn run(
    mut settings: MouseSettings,
    receiver: mpsc::Receiver<Message>,
    sender: EventSender,
    runtime: Arc<Mutex<Runtime>>,
    dpi_receiver: mpsc::Receiver<logitech::DpiRequest>,
) {
    let run_loop = CFRunLoop::get_current();
    let _ = sender.run_loop.set(run_loop.clone());
    run_loop.add_source(&sender.source.0, unsafe { kCFRunLoopDefaultMode });
    let logitech = logitech::Service::new(
        settings.clone(),
        runtime.clone(),
        sender.clone(),
        dpi_receiver,
    );
    let mut tap: Option<Tap> = None;
    let mut next_check = Instant::now();
    loop {
        let message = if tap.is_some() {
            match receiver.try_recv() {
                Ok(message) => Some(message),
                Err(mpsc::TryRecvError::Empty) => None,
                Err(mpsc::TryRecvError::Disconnected) => break,
            }
        } else {
            match receiver.recv_timeout(Duration::from_millis(250)) {
                Ok(message) => Some(message),
                Err(mpsc::RecvTimeoutError::Timeout) => None,
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        };
        let had_message = message.is_some();
        match message {
            Some(Message::Stop) => break,
            Some(Message::Configure(updated)) if settings != updated => {
                settings = updated;
                logitech.configure(settings.clone());
                if let Some(tap) = tap.as_mut() {
                    if !settings.enabled || !settings.smooth_scrolling {
                        tap.processor.motion.clear();
                    }
                    if !settings.enabled
                        || tap.processor.held.as_ref().is_some_and(|held| {
                            settings.button_action(held.button) != MouseAction::MissionControl
                        })
                    {
                        tap.processor.cancel_gesture();
                    }
                    tap.processor.settings = settings.clone();
                }
                next_check = Instant::now();
            }
            Some(Message::Wake) => {
                if let Some(tap) = tap.as_mut() {
                    tap.processor.cancel_gesture();
                }
                next_check = Instant::now();
            }
            Some(Message::LogitechButton { button, down }) => {
                if let Some(tap) = tap.as_mut() {
                    let result = if down {
                        if record_button(&runtime, button) || !settings.enabled {
                            Ok(())
                        } else {
                            tap.processor.press(button)
                        }
                    } else {
                        tap.processor.release(button)
                    };
                    if let Err(error) = result {
                        tap.processor.fault = Some(error);
                    }
                } else if down {
                    record_button(&runtime, button);
                }
            }
            Some(Message::LogitechDisconnected) => {
                if let Some(tap) = tap.as_mut() {
                    tap.processor.cancel_gesture();
                }
            }
            Some(Message::Configure(_)) | None => {}
        }
        if Instant::now() >= next_check {
            next_check = Instant::now() + Duration::from_millis(500);
            let capturing = runtime.lock().is_ok_and(|state| state.capturing());
            let held = tap
                .as_ref()
                .is_some_and(|tap| tap.processor.suppressed.iter().any(|held| *held));
            if (!settings.enabled && !capturing && !held) || !super::finder::accessibility_granted()
            {
                tap = None;
                publish(&runtime, false, None);
            } else {
                if tap
                    .as_ref()
                    .is_some_and(|tap| !tap.enabled() || tap.processor.fault.is_some())
                {
                    tap = None;
                }
                if tap.is_none() {
                    match Tap::new(settings.clone(), runtime.clone()) {
                        Ok(created) => tap = Some(created),
                        Err(error) => {
                            publish(&runtime, false, Some(error));
                            next_check = Instant::now() + Duration::from_secs(3);
                        }
                    }
                }
                if let Some(tap) = tap.as_ref() {
                    publish(&runtime, settings.enabled && tap.enabled(), None);
                }
            }
        }
        if let Some(tap) = tap.as_mut() {
            let timeout = if had_message {
                Duration::ZERO
            } else if tap.processor.motion.active() {
                Duration::from_millis(8).saturating_sub(tap.processor.last_frame.elapsed())
            } else {
                Duration::from_millis(250)
            };
            CFRunLoop::run_in_mode(unsafe { kCFRunLoopDefaultMode }, timeout, true);
            if let Err(error) = tap.processor.tick() {
                tap.processor.motion.clear();
                tap.processor.cancel_gesture();
                tap.processor.fault = Some(error);
            }
            if let Some(error) = tap.processor.fault.as_ref() {
                publish(&runtime, false, Some(error.clone()));
            }
        }
    }
    drop(tap);
    run_loop.remove_source(&sender.source.0, unsafe { kCFRunLoopDefaultMode });
    publish(&runtime, false, None);
}

struct HeldGesture {
    button: u16,
    gesture: Gesture,
    started: Instant,
}

struct Processor {
    settings: MouseSettings,
    motion: Motion,
    source: CGEventSource,
    pointer_source: CGEventSource,
    port: CFMachPortRef,
    movement_port: CFMachPortRef,
    suppressed: [bool; 30],
    held: Option<HeldGesture>,
    runtime: Arc<Mutex<Runtime>>,
    flags: CGEventFlags,
    last_frame: Instant,
    fault: Option<String>,
}

unsafe extern "C" fn callback(
    _proxy: *mut c_void,
    kind: u32,
    event: EventRef,
    info: *mut c_void,
) -> EventRef {
    unsafe { dispatch(kind, event, info, false) }
}

unsafe extern "C" fn movement_callback(
    _proxy: *mut c_void,
    kind: u32,
    event: EventRef,
    info: *mut c_void,
) -> EventRef {
    unsafe { dispatch(kind, event, info, true) }
}

unsafe fn dispatch(kind: u32, event: EventRef, info: *mut c_void, movement: bool) -> EventRef {
    let processor = unsafe { &mut *info.cast::<Processor>() };
    match catch_unwind(AssertUnwindSafe(|| {
        processor.handle_tap(kind, event, movement)
    })) {
        Ok(result) => result,
        Err(_) => {
            processor.motion.clear();
            processor.cancel_gesture();
            processor.fault = Some("鼠标事件处理失败，正在恢复".into());
            event
        }
    }
}

impl Processor {
    fn handle_tap(&mut self, kind: u32, event: EventRef, movement: bool) -> EventRef {
        if kind == DISABLED_TIMEOUT || kind == DISABLED_INPUT {
            self.motion.clear();
            let port = if movement {
                self.movement_port
            } else {
                self.port
            };
            let wanted = !movement || self.held.is_some();
            if wanted && !port.is_null() && !unsafe { CGEventTapIsEnabled(port) } {
                unsafe { CGEventTapEnable(port, true) };
            }
            return event;
        }
        self.handle(kind, event)
    }

    fn handle(&mut self, kind: u32, event: EventRef) -> EventRef {
        if event.is_null() {
            return event;
        }
        let integer = |field| unsafe { CGEventGetIntegerValueField(event, field) };
        if integer(EventField::EVENT_SOURCE_USER_DATA) == EVENT_MARKER {
            return event;
        }
        if matches!(kind, 5 | 6 | 7 | OTHER_DRAGGED) && self.held.is_some() {
            let x = integer(EventField::MOUSE_EVENT_DELTA_X) as f64;
            let y = integer(EventField::MOUSE_EVENT_DELTA_Y) as f64;
            if let Err(error) = self.gesture_movement(x, y) {
                self.fault = Some(error);
                self.cancel_gesture();
            }
            return ptr::null_mut();
        }
        if matches!(kind, OTHER_UP | OTHER_DRAGGED) {
            let button = integer(EventField::MOUSE_EVENT_BUTTON_NUMBER);
            if (2..=31).contains(&button) {
                let captured = &mut self.suppressed[(button - 2) as usize];
                let drop_event = *captured;
                if kind == OTHER_UP {
                    *captured = false;
                    if let Err(error) = self.release(button as u16 + 1) {
                        self.fault = Some(error);
                    }
                }
                if drop_event {
                    return ptr::null_mut();
                }
            }
            return event;
        }
        if kind == OTHER_DOWN {
            let button = integer(EventField::MOUSE_EVENT_BUTTON_NUMBER);
            if (2..=31).contains(&button) && self.suppressed[(button - 2) as usize] {
                return ptr::null_mut();
            }
        }
        if self.fault.is_some() {
            return event;
        }
        if kind == OTHER_DOWN {
            let button = integer(EventField::MOUSE_EVENT_BUTTON_NUMBER);
            if (2..=31).contains(&button) && record_button(&self.runtime, button as u16 + 1) {
                self.suppressed[(button - 2) as usize] = true;
                return ptr::null_mut();
            }
        }
        if !self.settings.enabled {
            return event;
        }
        if kind == SCROLL {
            let flags = unsafe { CGEventGetFlags(event) };
            let modified = flags.intersects(
                CGEventFlags::CGEventFlagCommand
                    | CGEventFlags::CGEventFlagControl
                    | CGEventFlags::CGEventFlagAlternate,
            );
            if integer(EventField::SCROLL_WHEEL_EVENT_IS_CONTINUOUS) != 0
                || integer(SCROLL_PHASE) != 0
                || integer(MOMENTUM_PHASE) != 0
                || integer(EventField::TABLET_EVENT_DEVICE_ID) != 0
                || modified
            {
                self.motion.clear();
                return event;
            }
            let vertical = if self.settings.reverse_vertical {
                -1.0
            } else {
                1.0
            } * self.settings.scroll_speed;
            let horizontal = if self.settings.reverse_horizontal {
                -1.0
            } else {
                1.0
            } * self.settings.scroll_speed;
            if self.settings.smooth_scrolling {
                let pixels = |point, line| {
                    let value = integer(point) as f64;
                    if value != 0.0 {
                        value
                    } else {
                        integer(line) as f64 * 12.0
                    }
                };
                let y = pixels(
                    EventField::SCROLL_WHEEL_EVENT_POINT_DELTA_AXIS_1,
                    EventField::SCROLL_WHEEL_EVENT_DELTA_AXIS_1,
                ) * vertical;
                let x = pixels(
                    EventField::SCROLL_WHEEL_EVENT_POINT_DELTA_AXIS_2,
                    EventField::SCROLL_WHEEL_EVENT_DELTA_AXIS_2,
                ) * horizontal;
                if x == 0.0 && y == 0.0 {
                    return event;
                }
                if !self.motion.active() {
                    self.last_frame = Instant::now();
                } else if self.flags != flags {
                    self.motion.clear();
                }
                self.flags = flags;
                self.motion.push(y, x);
                return ptr::null_mut();
            }
            for (line, fixed, point, scale) in [
                (
                    EventField::SCROLL_WHEEL_EVENT_DELTA_AXIS_1,
                    EventField::SCROLL_WHEEL_EVENT_FIXED_POINT_DELTA_AXIS_1,
                    EventField::SCROLL_WHEEL_EVENT_POINT_DELTA_AXIS_1,
                    vertical,
                ),
                (
                    EventField::SCROLL_WHEEL_EVENT_DELTA_AXIS_2,
                    EventField::SCROLL_WHEEL_EVENT_FIXED_POINT_DELTA_AXIS_2,
                    EventField::SCROLL_WHEEL_EVENT_POINT_DELTA_AXIS_2,
                    horizontal,
                ),
            ] {
                let line_delta = integer(line) as f64 * scale;
                let point_delta = integer(point) as f64 * scale;
                let fixed_delta = unsafe { CGEventGetDoubleValueField(event, fixed) } * scale;
                unsafe {
                    CGEventSetIntegerValueField(event, line, line_delta.round() as i64);
                    CGEventSetIntegerValueField(event, point, point_delta.round() as i64);
                    CGEventSetDoubleValueField(event, fixed, fixed_delta);
                }
            }
            return event;
        }
        if matches!(kind, 1 | 3 | OTHER_DOWN) {
            self.motion.clear();
        }
        if matches!(kind, 1 | 3) {
            self.cancel_gesture();
        }
        if kind != OTHER_DOWN {
            return event;
        }
        let button = integer(EventField::MOUSE_EVENT_BUTTON_NUMBER);
        if !(2..=31).contains(&button) {
            return event;
        }
        let index = (button - 2) as usize;
        let action = self.settings.button_action(button as u16 + 1);
        if action == MouseAction::Default {
            return event;
        }
        if let Err(error) = self.press(button as u16 + 1) {
            self.fault = Some(error);
            return event;
        }
        self.suppressed[index] = true;
        ptr::null_mut()
    }

    fn cancel_gesture(&mut self) {
        let was_held = self.held.take().is_some();
        if was_held && !self.movement_port.is_null() {
            unsafe { CGEventTapEnable(self.movement_port, false) };
        }
    }

    fn press(&mut self, button: u16) -> Result<(), String> {
        let action = self.settings.button_action(button);
        self.motion.clear();
        if action == MouseAction::MissionControl {
            self.cancel_gesture();
            self.held = Some(HeldGesture {
                button,
                gesture: Gesture::default(),
                started: Instant::now(),
            });
            if !self.movement_port.is_null() {
                unsafe { CGEventTapEnable(self.movement_port, true) };
                if !unsafe { CGEventTapIsEnabled(self.movement_port) } {
                    self.cancel_gesture();
                    return Err("无法启用鼠标手势监听，请重启 MTool".into());
                }
            }
            Ok(())
        } else {
            self.action(action)
        }
    }

    fn release(&mut self, button: u16) -> Result<(), String> {
        if self.held.as_ref().is_some_and(|held| held.button == button) {
            let clicked = self.held.as_ref().unwrap().gesture.clicked();
            self.cancel_gesture();
            if clicked && self.settings.enabled && self.fault.is_none() {
                return self.action(MouseAction::MissionControl);
            }
        }
        Ok(())
    }

    fn gesture_movement(&mut self, x: f64, y: f64) -> Result<(), String> {
        if let Some(direction) = self
            .held
            .as_mut()
            .and_then(|held| held.gesture.movement(x, y))
        {
            let key = match direction {
                Direction::Left => KeyCode::RIGHT_ARROW,
                Direction::Right => KeyCode::LEFT_ARROW,
                Direction::Up => KeyCode::UP_ARROW,
                Direction::Down => KeyCode::DOWN_ARROW,
            };
            self.key_action(key, CGEventFlags::CGEventFlagControl)?;
        }
        Ok(())
    }

    fn action(&self, action: MouseAction) -> Result<(), String> {
        use MouseAction::*;
        let (key, flags) = match action {
            Default | None => return Ok(()),
            Back => (KeyCode::ANSI_LEFT_BRACKET, CGEventFlags::CGEventFlagCommand),
            Forward => (
                KeyCode::ANSI_RIGHT_BRACKET,
                CGEventFlags::CGEventFlagCommand,
            ),
            MissionControl => (KeyCode::UP_ARROW, CGEventFlags::CGEventFlagControl),
            ShowDesktop => (KeyCode::F11, CGEventFlags::empty()),
            QuickLook => (KeyCode::SPACE, CGEventFlags::empty()),
        };
        self.key_action(key, flags)
    }

    fn key_action(&self, key: u16, mut flags: CGEventFlags) -> Result<(), String> {
        if [
            KeyCode::LEFT_ARROW,
            KeyCode::RIGHT_ARROW,
            KeyCode::UP_ARROW,
            KeyCode::DOWN_ARROW,
        ]
        .contains(&key)
        {
            flags |= CGEventFlags::CGEventFlagSecondaryFn | CGEventFlags::CGEventFlagNumericPad;
        }
        let down = CGEvent::new_keyboard_event(self.source.clone(), key, true)
            .map_err(|_| "无法创建鼠标按键动作".to_string())?;
        let up = CGEvent::new_keyboard_event(self.source.clone(), key, false)
            .map_err(|_| "无法创建鼠标按键动作".to_string())?;
        for event in [down, up] {
            event.set_flags(flags);
            event.set_integer_value_field(EventField::EVENT_SOURCE_USER_DATA, EVENT_MARKER);
            event.post(CGEventTapLocation::Session);
        }
        Ok(())
    }

    fn tick(&mut self) -> Result<(), String> {
        if self
            .held
            .as_ref()
            .is_some_and(|held| held.started.elapsed() > Duration::from_secs(30))
        {
            self.cancel_gesture();
        }
        let elapsed = self.last_frame.elapsed();
        if elapsed < Duration::from_millis(8) || !self.motion.active() {
            return Ok(());
        }
        self.last_frame = Instant::now();
        if elapsed > Duration::from_millis(150) || !unsafe { CGEventTapIsEnabled(self.port) } {
            self.motion.clear();
            return Ok(());
        }
        let (vertical, horizontal) = self.motion.step(
            elapsed.as_secs_f64() * 1000.0,
            self.settings.smooth_duration_ms,
        );
        if vertical == 0 && horizontal == 0 {
            return Ok(());
        }
        let event = CGEvent::new_scroll_event(
            self.source.clone(),
            ScrollEventUnit::PIXEL,
            2,
            vertical,
            horizontal,
            0,
        )
        .map_err(|_| "无法生成平滑滚动事件".to_string())?;
        let pointer = CGEvent::new(self.pointer_source.clone())
            .map_err(|_| "无法读取当前鼠标位置".to_string())?;
        event.set_location(pointer.location());
        event.set_flags(self.flags);
        event.set_integer_value_field(EventField::EVENT_SOURCE_USER_DATA, EVENT_MARKER);
        event.post(CGEventTapLocation::Session);
        Ok(())
    }
}
