use core_foundation::{
    array::CFArray,
    base::{CFType, CFTypeID, CFTypeRef, TCFType},
    boolean::CFBoolean,
    dictionary::{CFDictionary, CFDictionaryRef},
    string::{CFString, CFStringRef},
    url::CFURL,
};
use core_graphics::{
    event::{CGEvent, CGEventFlags},
    event_source::{CGEventSource, CGEventSourceStateID},
};
use mtool_core::{AppError, Result};
use objc2_app_kit::{NSRunningApplication, NSWorkspace};
use objc2_foundation::{NSArray, NSString, NSURL};
use std::{
    path::Path,
    thread,
    time::{Duration, Instant},
};

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn AXIsProcessTrusted() -> u8;
    fn AXIsProcessTrustedWithOptions(options: CFDictionaryRef) -> u8;
    static kAXTrustedCheckOptionPrompt: CFStringRef;
    fn AXUIElementCreateApplication(pid: i32) -> CFTypeRef;
    fn AXUIElementGetTypeID() -> CFTypeID;
    fn AXUIElementSetMessagingTimeout(element: CFTypeRef, seconds: f32) -> i32;
    fn AXUIElementCopyAttributeValue(
        element: CFTypeRef,
        attribute: CFStringRef,
        value: *mut CFTypeRef,
    ) -> i32;
}

pub fn accessibility_granted() -> bool {
    unsafe { AXIsProcessTrusted() != 0 }
}

fn request_accessibility() {
    let key = unsafe { CFString::wrap_under_get_rule(kAXTrustedCheckOptionPrompt) };
    let options =
        CFDictionary::from_CFType_pairs(&[(key.as_CFType(), CFBoolean::true_value().as_CFType())]);
    unsafe {
        AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef());
    }
}

pub fn require_accessibility() -> Result<()> {
    if accessibility_granted() {
        return Ok(());
    }
    request_accessibility();
    Err(AppError::new(
        "accessibility_required",
        "请在辅助功能中授权 MTool 后重试",
    ))
}

pub fn open_accessibility_settings() -> Result<()> {
    request_accessibility();
    let url = NSURL::URLWithString(&NSString::from_str(
        "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility",
    ))
    .ok_or_else(|| AppError::new("settings_unavailable", "无法打开辅助功能设置"))?;
    if !NSWorkspace::sharedWorkspace().openURL(&url) {
        return Err(AppError::new(
            "settings_unavailable",
            "无法打开辅助功能设置",
        ));
    }
    Ok(())
}

pub fn select(path: &Path) {
    let url = NSURL::fileURLWithPath(&NSString::from_str(&path.to_string_lossy()));
    NSWorkspace::sharedWorkspace()
        .activateFileViewerSelectingURLs(&NSArray::from_retained_slice(&[url]));
}

pub fn rename_selected(path: &Path) -> Result<()> {
    let apps = NSRunningApplication::runningApplicationsWithBundleIdentifier(&NSString::from_str(
        "com.apple.finder",
    ));
    let finder = apps.firstObject().ok_or_else(rename_unavailable)?;
    let pid = finder.processIdentifier();
    let application = unsafe { AXUIElementCreateApplication(pid) };
    if application.is_null() {
        return Err(rename_unavailable());
    }
    let application = unsafe { CFType::wrap_under_create_rule(application) };
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut was_active = false;
    while Instant::now() < deadline {
        if finder_is_active(pid) {
            was_active = true;
            if selection_matches(&application, path, deadline) && finder_is_active(pid) {
                return send_rename_key(pid);
            }
        } else if was_active {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(50));
    }
    if finder_is_active(pid) {
        Err(rename_unavailable())
    } else {
        Ok(())
    }
}

fn finder_is_active(pid: i32) -> bool {
    NSWorkspace::sharedWorkspace()
        .frontmostApplication()
        .is_some_and(|app| app.processIdentifier() == pid)
}

fn rename_unavailable() -> AppError {
    AppError::new(
        "finder_rename_unavailable",
        "文件已创建，未能进入重命名；可在访达中选中文件后按回车",
    )
}

fn attribute(element: &CFType, name: &str) -> Option<CFType> {
    let name = CFString::new(name);
    let mut value = std::ptr::null();
    if unsafe { AXUIElementSetMessagingTimeout(element.as_CFTypeRef(), 0.1) } != 0 {
        return None;
    }
    let code = unsafe {
        AXUIElementCopyAttributeValue(
            element.as_CFTypeRef(),
            name.as_concrete_TypeRef(),
            &mut value,
        )
    };
    if code != 0 || value.is_null() {
        return None;
    }
    Some(unsafe { CFType::wrap_under_create_rule(value) })
}

fn string_attribute(element: &CFType, name: &str) -> Option<String> {
    attribute(element, name)?
        .downcast::<CFString>()
        .map(|value| value.to_string())
}

fn element_attribute(element: &CFType, name: &str) -> Option<CFType> {
    attribute(element, name).filter(is_element)
}

fn is_element(value: &CFType) -> bool {
    value.type_of() == unsafe { AXUIElementGetTypeID() }
}

fn elements_attribute(element: &CFType, name: &str) -> Option<Vec<CFType>> {
    let array = attribute(element, name)?.downcast::<CFArray>()?;
    if array.len() > 64 {
        return None;
    }
    Some(
        array
            .iter()
            .filter_map(|value| {
                if (*value).is_null() {
                    return None;
                }
                let value = unsafe { CFType::wrap_under_get_rule(*value) };
                is_element(&value).then_some(value)
            })
            .collect(),
    )
}

fn url_matches(value: CFType, path: &Path) -> bool {
    let candidate = value
        .downcast::<CFURL>()
        .and_then(|url| url.to_path())
        .or_else(|| {
            value.downcast::<CFString>().and_then(|url| {
                tauri::Url::parse(&url.to_string())
                    .ok()?
                    .to_file_path()
                    .ok()
            })
        });
    candidate.is_some_and(|candidate| {
        candidate == path
            || std::fs::canonicalize(candidate).is_ok_and(|candidate| candidate == path)
    })
}

fn selection_matches(application: &CFType, path: &Path, deadline: Instant) -> bool {
    let Some(mut focused) = element_attribute(application, "AXFocusedUIElement") else {
        return false;
    };
    if matches!(
        string_attribute(&focused, "AXRole").as_deref(),
        Some("AXTextField" | "AXTextArea" | "AXComboBox")
    ) {
        return false;
    }
    let directory_matches =
        element_attribute(application, "AXFocusedWindow").is_some_and(|window| {
            path.parent().is_some_and(|parent| {
                ["AXDocument", "AXURL"].iter().any(|name| {
                    attribute(&window, name).is_some_and(|url| url_matches(url, parent))
                })
            })
        });
    for _ in 0..6 {
        if Instant::now() >= deadline {
            return false;
        }
        for name in ["AXSelectedChildren", "AXSelectedRows"] {
            if let Some(selected) = elements_attribute(&focused, name) {
                if selected.len() == 1
                    && selected_item_matches(&selected[0], path, directory_matches, deadline)
                {
                    return true;
                }
            }
        }
        let Some(parent) = element_attribute(&focused, "AXParent") else {
            break;
        };
        focused = parent;
    }
    false
}

fn selected_item_matches(
    element: &CFType,
    path: &Path,
    directory_matches: bool,
    deadline: Instant,
) -> bool {
    let mut pending = vec![(element.clone(), 0)];
    let mut remaining = 64;
    while let Some((item, depth)) = pending.pop() {
        if Instant::now() >= deadline || remaining == 0 {
            return false;
        }
        remaining -= 1;
        if attribute(&item, "AXURL").is_some_and(|url| url_matches(url, path)) {
            return true;
        }
        if directory_matches
            && ["AXValue", "AXTitle"].iter().any(|name| {
                string_attribute(&item, name)
                    .is_some_and(|name| path.file_name().is_some_and(|file| file == name.as_str()))
            })
        {
            return true;
        }
        if depth < 5 {
            if let Some(children) = elements_attribute(&item, "AXChildren") {
                pending.extend(children.into_iter().map(|child| (child, depth + 1)));
            }
        }
    }
    false
}

fn send_rename_key(pid: i32) -> Result<()> {
    let source =
        CGEventSource::new(CGEventSourceStateID::Private).map_err(|_| rename_unavailable())?;
    let down =
        CGEvent::new_keyboard_event(source.clone(), 36, true).map_err(|_| rename_unavailable())?;
    let up = CGEvent::new_keyboard_event(source, 36, false).map_err(|_| rename_unavailable())?;
    down.set_flags(CGEventFlags::empty());
    up.set_flags(CGEventFlags::empty());
    down.post_to_pid(pid);
    up.post_to_pid(pid);
    Ok(())
}
