use crate::state::{lock, AppState};
use std::{
    sync::{atomic::Ordering, Arc},
    thread,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter};

pub fn start(app: AppHandle, state: Arc<AppState>) {
    thread::Builder::new()
        .name("clipboard-monitor".into())
        .spawn(move || {
            let watcher = mtool_platform::ClipboardWatcher::new().ok();
            let mut previous = mtool_platform::clipboard_revision();
            let mut prune_at = Instant::now();
            let mut retries = 0;
            while !state.stop.load(Ordering::Relaxed) {
                if let Some(watcher) = &watcher {
                    watcher.wait();
                } else {
                    thread::sleep(Duration::from_millis(300));
                }
                let Ok(p) = lock(&state.persistence) else {
                    break;
                };
                let enabled = p.store.is_some() && p.settings.capture_enabled;
                drop(p);
                let Ok(_io) = lock(&state.clipboard_io) else {
                    break;
                };
                let revision = mtool_platform::clipboard_revision();
                if !enabled {
                    previous = revision;
                    continue;
                }
                if prune_at.elapsed() >= Duration::from_secs(60) {
                    if let Ok(p) = lock(&state.persistence) {
                        if let Some(store) = &p.store {
                            if let Err(error) =
                                store.prune(&p.settings, chrono::Utc::now().timestamp_millis())
                            {
                                if let Ok(mut message) = lock(&state.capture_error) {
                                    *message = Some(error.message);
                                }
                            }
                        }
                    }
                    prune_at = Instant::now();
                    let _ = app.emit("clipboard-changed", ());
                }
                if revision == previous {
                    continue;
                }
                if lock(&state.own_revision).is_ok_and(|own| *own == Some(revision)) {
                    previous = revision;
                    continue;
                }
                let source = mtool_platform::clipboard_source();
                let excluded = state
                    .settings()
                    .is_ok_and(|s| mtool_clipboard::excluded(source.as_deref(), &s));
                if excluded || mtool_platform::is_sensitive() {
                    previous = revision;
                    continue;
                }
                let result = mtool_platform::read_clipboard();
                if revision != mtool_platform::clipboard_revision() {
                    continue;
                }
                if result.as_ref().is_err_and(|e| e.code == "clipboard_busy") && retries < 2 {
                    retries += 1;
                    continue;
                }
                previous = revision;
                retries = 0;
                let result = result.and_then(|content| {
                    let Some(content) = content else {
                        return Ok(false);
                    };
                    let mut p = lock(&state.persistence)?;
                    let settings = p.settings.clone();
                    let Some(store) = p.store.as_mut() else {
                        return Ok(false);
                    };
                    mtool_clipboard::capture(
                        store,
                        &content,
                        source.as_deref(),
                        &settings,
                        chrono::Utc::now().timestamp_millis(),
                    )
                });
                let message = result.as_ref().err().map(|e| e.message.clone());
                if let Ok(mut error) = lock(&state.capture_error) {
                    if *error != message {
                        *error = message;
                        let _ = app.emit("status-changed", ());
                    }
                }
                if matches!(result, Ok(true)) {
                    let _ = app.emit("clipboard-changed", ());
                }
            }
        })
        .expect("clipboard monitor thread can start");
}
