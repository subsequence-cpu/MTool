use windows::{
    core::w,
    Win32::{
        Foundation::CloseHandle,
        System::{
            DataExchange::{
                GetClipboardOwner, GetClipboardSequenceNumber, IsClipboardFormatAvailable,
                RegisterClipboardFormatW,
            },
            Threading::{
                OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
                PROCESS_QUERY_LIMITED_INFORMATION,
            },
        },
        UI::WindowsAndMessaging::GetWindowThreadProcessId,
    },
};

pub fn clipboard_revision() -> u64 {
    unsafe { u64::from(GetClipboardSequenceNumber()) }
}
pub fn mark_owned() {}
pub fn is_sensitive() -> bool {
    unsafe {
        let format = RegisterClipboardFormatW(w!("ExcludeClipboardContentFromMonitorProcessing"));
        format != 0 && IsClipboardFormatAvailable(format).is_ok()
    }
}
pub fn clipboard_source() -> Option<String> {
    unsafe {
        let window = GetClipboardOwner().ok()?;
        let mut pid = 0;
        GetWindowThreadProcessId(window, Some(&mut pid));
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buffer = [0u16; 1024];
        let mut length = buffer.len() as u32;
        let result = QueryFullProcessImageNameW(
            process,
            PROCESS_NAME_WIN32,
            windows::core::PWSTR(buffer.as_mut_ptr()),
            &mut length,
        );
        let _ = CloseHandle(process);
        if result.is_err() {
            return None;
        }
        let path = String::from_utf16_lossy(&buffer[..length as usize]);
        Some(path.rsplit(['\\', '/']).next()?.to_owned())
    }
}
