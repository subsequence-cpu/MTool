use crate::{state::AppState, windows};
use mtool_core::{AppError, Result};
use objc2::{
    define_class, msg_send, rc::Retained, runtime::ProtocolObject, DefinedClass, MainThreadMarker,
    MainThreadOnly, Message,
};
use objc2_app_kit::{
    NSApplication, NSAutoresizingMaskOptions, NSBackingStoreType, NSEvent, NSEventType, NSPanel,
    NSResponder, NSScreen, NSView, NSWindowCollectionBehavior, NSWindowDelegate, NSWindowStyleMask,
};
use objc2_foundation::{NSNotification, NSObject, NSObjectProtocol, NSPoint, NSRect, NSSize};
use std::{
    cell::{Cell, RefCell},
    panic::AssertUnwindSafe,
    sync::{atomic::Ordering, Arc},
};
use tauri::{AppHandle, Manager, WebviewWindow};

struct PanelIvars {
    interactive: bool,
    mouse_down: RefCell<Option<Retained<NSEvent>>>,
}

define_class!(
    #[unsafe(super(NSPanel))]
    #[name = "MToolFloatingPanel"]
    #[thread_kind = MainThreadOnly]
    #[ivars = PanelIvars]
    struct FloatingPanel;
    unsafe impl NSObjectProtocol for FloatingPanel {}
    impl FloatingPanel {
        #[unsafe(method(canBecomeKeyWindow))]
        fn can_become_key(&self) -> bool { self.ivars().interactive }
        #[unsafe(method(canBecomeMainWindow))]
        fn can_become_main(&self) -> bool { false }
        #[unsafe(method(sendEvent:))]
        fn send_event(&self, event: &NSEvent) {
            if event.r#type() == NSEventType::LeftMouseDown {
                *self.ivars().mouse_down.borrow_mut() = Some(event.retain());
            }
            unsafe { let _: () = msg_send![super(self), sendEvent: event]; }
        }
    }
);

struct DelegateIvars {
    app: AppHandle,
    interactive: bool,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[name = "MToolFloatingPanelDelegate"]
    #[thread_kind = MainThreadOnly]
    #[ivars = DelegateIvars]
    struct PanelDelegate;
    unsafe impl NSObjectProtocol for PanelDelegate {}
    unsafe impl NSWindowDelegate for PanelDelegate {
        #[unsafe(method(windowDidBecomeKey:))]
        fn became_key(&self, _notification: &NSNotification) {
            if self.ivars().interactive {
                if let Some(state) = self.ivars().app.try_state::<Arc<AppState>>() {
                    state.clipboard_focused.store(true, Ordering::Relaxed);
                }
            }
        }
        #[unsafe(method(windowDidResignKey:))]
        fn resigned_key(&self, _notification: &NSNotification) {
            if !self.ivars().interactive {
                return;
            }
            let app = self.ivars().app.clone();
            let queued = app.clone();
            let _ = app.run_on_main_thread(move || {
                if !is_focused().unwrap_or(true) {
                    if let Some(state) = queued.try_state::<Arc<AppState>>() {
                        if !state.stop.load(Ordering::Relaxed)
                            && state.clipboard_focused.swap(false, Ordering::Relaxed)
                        {
                            let _ = windows::hide_clipboard(&queued);
                        }
                    }
                }
            });
        }
        #[unsafe(method(windowDidMove:))]
        fn moved(&self, _notification: &NSNotification) {
            self.sync_preview();
        }
        #[unsafe(method(windowDidResize:))]
        fn resized(&self, _notification: &NSNotification) {
            self.sync_preview();
        }
        #[unsafe(method(windowShouldClose:))]
        fn should_close(&self, _sender: &objc2_app_kit::NSWindow) -> bool {
            if self.ivars().interactive {
                let _ = windows::hide_clipboard(&self.ivars().app);
            }
            false
        }
    }
);

impl PanelDelegate {
    fn sync_preview(&self) {
        if self.ivars().interactive {
            let _ = windows::sync_preview(&self.ivars().app);
        }
    }
}

struct HostedPanel {
    panel: Retained<FloatingPanel>,
    delegate: Retained<PanelDelegate>,
    host: WebviewWindow,
    content: Retained<NSView>,
    responder: Option<Retained<NSResponder>>,
    positioned: Cell<bool>,
}

impl HostedPanel {
    fn new(app: &AppHandle, label: &str, interactive: bool, mtm: MainThreadMarker) -> Result<Self> {
        let host = app.get_webview_window(label).ok_or_else(unavailable)?;
        let pointer = host.ns_window().map_err(|_| unavailable())?;
        let native_host = unsafe { pointer.cast::<objc2_app_kit::NSWindow>().as_ref() }
            .ok_or_else(unavailable)?;
        let content = native_host.contentView().ok_or_else(unavailable)?;
        let responder = native_host.firstResponder();
        let bounds = content.bounds();
        let mask = NSWindowStyleMask::NonactivatingPanel
            | if interactive {
                NSWindowStyleMask::Resizable
            } else {
                NSWindowStyleMask::empty()
            };
        let allocated = FloatingPanel::alloc(mtm).set_ivars(PanelIvars {
            interactive,
            mouse_down: RefCell::new(None),
        });
        let panel: Retained<FloatingPanel> = unsafe {
            msg_send![super(allocated), initWithContentRect: bounds, styleMask: mask,
                backing: NSBackingStoreType::Buffered, defer: false]
        };
        unsafe {
            panel.setReleasedWhenClosed(false);
        }
        panel.setFloatingPanel(true);
        panel.setHidesOnDeactivate(false);
        panel.setBecomesKeyOnlyIfNeeded(false);
        panel.setCollectionBehavior(
            NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::CanJoinAllApplications
                | NSWindowCollectionBehavior::FullScreenAuxiliary
                | NSWindowCollectionBehavior::IgnoresCycle,
        );
        panel.setLevel(native_host.level());
        panel.setBackgroundColor(Some(&native_host.backgroundColor()));
        panel.setHasShadow(interactive);
        panel.setIgnoresMouseEvents(!interactive);
        if interactive {
            panel.setContentMinSize(native_host.contentMinSize());
        }
        let allocated = PanelDelegate::alloc(mtm).set_ivars(DelegateIvars {
            app: app.clone(),
            interactive,
        });
        let delegate: Retained<PanelDelegate> = unsafe { msg_send![super(allocated), init] };
        let hosted = Self {
            panel,
            delegate,
            host,
            content,
            responder,
            positioned: Cell::new(false),
        };
        let placeholder = NSView::initWithFrame(NSView::alloc(mtm), bounds);
        native_host.setContentView(Some(&placeholder));
        hosted.content.setAutoresizingMask(
            NSAutoresizingMaskOptions::ViewWidthSizable
                | NSAutoresizingMaskOptions::ViewHeightSizable,
        );
        hosted.panel.setContentView(Some(&hosted.content));
        hosted
            .panel
            .setDelegate(Some(ProtocolObject::from_ref(&*hosted.delegate)));
        Ok(hosted)
    }
}

impl Drop for HostedPanel {
    fn drop(&mut self) {
        let _ = objc2::exception::catch(AssertUnwindSafe(|| {
            self.panel.setDelegate(None);
            self.panel.ivars().mouse_down.borrow_mut().take();
            self.panel.orderOut(None);
            self.panel.setContentView(None);
            if let Ok(pointer) = self.host.ns_window() {
                if let Some(host) = unsafe { pointer.cast::<objc2_app_kit::NSWindow>().as_ref() } {
                    host.setContentView(Some(&self.content));
                }
            }
        }));
    }
}

struct Panels {
    clipboard: HostedPanel,
    preview: HostedPanel,
}
thread_local! { static PANELS: RefCell<Option<Panels>> = const { RefCell::new(None) }; }

fn unavailable() -> AppError {
    AppError::new("window_unavailable", "快捷浮窗不可用")
}
fn main_thread() -> Result<MainThreadMarker> {
    MainThreadMarker::new().ok_or_else(unavailable)
}
fn panel(interactive: bool) -> Result<Retained<FloatingPanel>> {
    main_thread()?;
    PANELS.with_borrow(|panels| {
        let panels = panels.as_ref().ok_or_else(unavailable)?;
        Ok(if interactive {
            panels.clipboard.panel.clone()
        } else {
            panels.preview.panel.clone()
        })
    })
}
fn guarded<T>(action: impl FnOnce() -> Result<T>) -> Result<T> {
    main_thread()?;
    objc2::exception::catch(AssertUnwindSafe(action))
        .map_err(|_| AppError::new("window_unavailable", "系统拒绝了浮窗操作"))?
}

pub fn prepare(app: &AppHandle) -> Result<()> {
    guarded(|| {
        let mtm = main_thread()?;
        let clipboard = HostedPanel::new(app, "clipboard", true, mtm)?;
        let preview = HostedPanel::new(app, "clipboard-preview", false, mtm)?;
        PANELS.with_borrow_mut(|panels| *panels = Some(Panels { clipboard, preview }));
        Ok(())
    })
}

pub fn show_clipboard() -> Result<()> {
    guarded(|| {
        let popup = panel(true)?;
        let (center, responder) = PANELS.with_borrow(|panels| {
            let hosted = &panels.as_ref().unwrap().clipboard;
            (!hosted.positioned.replace(true), hosted.responder.clone())
        });
        if center {
            let screens = NSScreen::screens(main_thread()?);
            let mouse = NSEvent::mouseLocation();
            if let Some(screen) = screens.iter().find(|screen| {
                let frame = screen.frame();
                mouse.x >= frame.origin.x
                    && mouse.x < frame.origin.x + frame.size.width
                    && mouse.y >= frame.origin.y
                    && mouse.y < frame.origin.y + frame.size.height
            }) {
                let area = screen.visibleFrame();
                let size = popup.frame().size;
                popup.setFrameOrigin(NSPoint::new(
                    area.origin.x + (area.size.width - size.width) / 2.0,
                    area.origin.y + (area.size.height - size.height) / 2.0,
                ));
            } else {
                popup.center();
            }
        }
        let application = NSApplication::sharedApplication(main_thread()?);
        if application.isHidden() {
            application.unhideWithoutActivation();
        }
        popup.orderFrontRegardless();
        popup.makeKeyWindow();
        popup.makeFirstResponder(responder.as_deref());
        Ok(())
    })
}
pub fn hide_clipboard() -> Result<()> {
    guarded(|| {
        let popup = panel(true)?;
        popup.ivars().mouse_down.borrow_mut().take();
        popup.orderOut(None);
        Ok(())
    })
}
pub fn hide_preview() -> Result<()> {
    guarded(|| {
        panel(false)?.orderOut(None);
        Ok(())
    })
}
pub fn show_preview() -> Result<()> {
    guarded(|| {
        panel(false)?.orderFrontRegardless();
        Ok(())
    })
}
pub fn is_visible() -> Result<bool> {
    guarded(|| Ok(panel(true)?.isVisible()))
}
pub fn is_focused() -> Result<bool> {
    guarded(|| Ok(panel(true)?.isKeyWindow()))
}
pub fn preview_visible() -> Result<bool> {
    guarded(|| Ok(panel(false)?.isVisible()))
}

pub fn drag_clipboard() -> Result<()> {
    guarded(|| {
        let popup = panel(true)?;
        let event = popup.ivars().mouse_down.borrow_mut().take();
        if NSEvent::pressedMouseButtons() & 1 != 0 {
            if let Some(event) = event {
                popup.performWindowDragWithEvent(&event);
            }
        }
        Ok(())
    })
}

pub fn position_preview(width: u32, height: u32) -> Result<bool> {
    guarded(|| {
        let popup = panel(true)?;
        if !popup.isVisible() || width == 0 || height == 0 {
            return Ok(false);
        }
        let Some(screen) = popup.screen() else {
            return Ok(false);
        };
        let anchor = popup.frame();
        let area = screen.visibleFrame();
        let gap = 10.0;
        let right = anchor.origin.x + anchor.size.width + gap;
        let right_space = (area.origin.x + area.size.width - gap - right).max(0.0);
        let left_space = (anchor.origin.x - area.origin.x - 2.0 * gap).max(0.0);
        let use_right = right_space >= 320.0 || right_space >= left_space;
        let max_width = 320.0_f64.min(if use_right { right_space } else { left_space });
        let max_height = anchor
            .size
            .height
            .min((area.size.height - 2.0 * gap).max(0.0));
        if max_width < 48.0 || max_height <= 0.0 {
            return Ok(false);
        }
        let scale = popup.backingScaleFactor();
        let ratio = (max_width / f64::from(width))
            .min(max_height / f64::from(height))
            .min(1.0 / scale);
        let size = NSSize::new(f64::from(width) * ratio, f64::from(height) * ratio);
        let x = if use_right {
            right
        } else {
            anchor.origin.x - gap - size.width
        };
        let y = (anchor.origin.y + (anchor.size.height - size.height) / 2.0).clamp(
            area.origin.y + gap,
            area.origin.y + area.size.height - gap - size.height,
        );
        panel(false)?.setFrame_display(NSRect::new(NSPoint::new(x, y), size), true);
        Ok(true)
    })
}

pub fn shutdown() {
    if main_thread().is_ok() {
        let owned = PANELS.with_borrow_mut(Option::take);
        drop(owned);
    }
}
