use mtool_core::{AppError, Result};
use std::{
    sync::mpsc::{self, Receiver},
    thread::{self, JoinHandle},
    time::Duration,
};
use windows::{
    core::w,
    Win32::{
        Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM},
        System::{
            DataExchange::{AddClipboardFormatListener, RemoveClipboardFormatListener},
            LibraryLoader::GetModuleHandleW,
            Threading::GetCurrentThreadId,
        },
        UI::WindowsAndMessaging::{
            CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW,
            PostThreadMessageW, RegisterClassW, UnregisterClassW, HWND_MESSAGE, MSG,
            WM_CLIPBOARDUPDATE, WM_QUIT, WNDCLASSW,
        },
    },
};

unsafe extern "system" fn window_proc(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe { DefWindowProcW(window, message, wparam, lparam) }
}

pub struct ClipboardWatcher {
    receiver: Receiver<()>,
    thread_id: u32,
    thread: Option<JoinHandle<()>>,
}

impl ClipboardWatcher {
    pub fn new() -> Result<Self> {
        let (sender, receiver) = mpsc::sync_channel(1);
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let thread = thread::Builder::new()
            .name("clipboard-notifications".into())
            .spawn(move || {
                let initialized = (|| -> windows::core::Result<_> {
                    unsafe {
                        let instance = HINSTANCE(GetModuleHandleW(None)?.0);
                        let class = WNDCLASSW {
                            hInstance: instance,
                            lpszClassName: w!("MToolClipboardListener"),
                            lpfnWndProc: Some(window_proc),
                            ..Default::default()
                        };
                        if RegisterClassW(&class) == 0 {
                            return Err(windows::core::Error::from_win32());
                        }
                        let window = match CreateWindowExW(
                            Default::default(),
                            w!("MToolClipboardListener"),
                            w!(""),
                            Default::default(),
                            0,
                            0,
                            0,
                            0,
                            Some(HWND_MESSAGE),
                            None,
                            Some(instance),
                            None,
                        ) {
                            Ok(window) => window,
                            Err(error) => {
                                let _ =
                                    UnregisterClassW(w!("MToolClipboardListener"), Some(instance));
                                return Err(error);
                            }
                        };
                        if let Err(error) = AddClipboardFormatListener(window) {
                            let _ = DestroyWindow(window);
                            let _ = UnregisterClassW(w!("MToolClipboardListener"), Some(instance));
                            return Err(error);
                        }
                        Ok((window, instance, GetCurrentThreadId()))
                    }
                })();
                match initialized {
                    Ok((window, instance, id)) => unsafe {
                        if ready_tx.send(Ok(id)).is_ok() {
                            let mut message = MSG::default();
                            while GetMessageW(&mut message, None, 0, 0).0 > 0 {
                                if message.message == WM_CLIPBOARDUPDATE {
                                    let _ = sender.try_send(());
                                }
                                let _ = DispatchMessageW(&message);
                            }
                        }
                        let _ = RemoveClipboardFormatListener(window);
                        let _ = DestroyWindow(window);
                        let _ = UnregisterClassW(w!("MToolClipboardListener"), Some(instance));
                    },
                    Err(_) => {
                        let _ = ready_tx.send(Err(()));
                    }
                }
            })
            .map_err(|_| {
                AppError::new(
                    "clipboard_notifications_unavailable",
                    "无法启动系统剪贴板通知线程",
                )
            })?;
        match ready_rx.recv_timeout(Duration::from_secs(3)) {
            Ok(Ok(thread_id)) => Ok(Self {
                receiver,
                thread_id,
                thread: Some(thread),
            }),
            _ => Err(AppError::new(
                "clipboard_notifications_unavailable",
                "系统剪贴板通知不可用，将使用变更计数检测",
            )),
        }
    }

    pub fn wait(&self) {
        let _ = self.receiver.recv_timeout(Duration::from_secs(1));
    }
}

impl Drop for ClipboardWatcher {
    fn drop(&mut self) {
        unsafe {
            let _ = PostThreadMessageW(self.thread_id, WM_QUIT, WPARAM(0), LPARAM(0));
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
