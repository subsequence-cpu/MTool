use crate::common::{context_menu::ContextMenuState, state::lock};
use mtool_core::{AppError, ContextMenuStatus, Result};
use std::{
    fs::OpenOptions,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
};
use tauri::{AppHandle, Emitter, Manager};

#[derive(Default)]
pub(crate) struct State {
    file_action: Mutex<()>,
    active_jobs: Arc<AtomicUsize>,
}

pub(crate) fn settings() -> Result<()> {
    unsafe {
        let _: () = objc2::msg_send![
            objc2::class!(FIFinderSyncController),
            showExtensionManagementInterface
        ];
    }
    Ok(())
}

pub(crate) fn accessibility_settings() -> Result<()> {
    super::finder::open_accessibility_settings()
}

fn create_file(directory: &Path) -> Result<PathBuf> {
    if !directory.is_absolute() || !directory.is_dir() {
        return Err(AppError::invalid("目标文件夹不存在"));
    }
    for index in 1..=10_000 {
        let name = if index == 1 {
            "未命名.txt".to_owned()
        } else {
            format!("未命名 {index}.txt")
        };
        let path = directory.join(name);
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(_) => return Ok(path),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(match error.kind() {
                    std::io::ErrorKind::PermissionDenied => {
                        AppError::new("permission_denied", "无权在此文件夹新增文件")
                    }
                    _ => AppError::new("create_file_failed", format!("新增文件失败：{error}")),
                });
            }
        }
    }
    Err(AppError::new("file_name_exhausted", "未命名文件过多"))
}

fn create_in_finder(app: &AppHandle, directory: &Path) -> Result<()> {
    let state = app.state::<ContextMenuState>();
    let _action = lock(&state.platform.file_action)?;
    super::finder::require_accessibility()?;
    let path = create_file(directory)?;
    let selected = path.clone();
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    app.run_on_main_thread(move || {
        super::finder::select(&selected);
        let _ = sender.send(());
    })
    .map_err(|_| AppError::new("finder_unavailable", "文件已创建，无法在访达中选中"))?;
    receiver
        .recv()
        .map_err(|_| AppError::new("finder_unavailable", "文件已创建，无法在访达中选中"))?;
    super::finder::rename_selected(&path)
}

#[link(name = "FinderSync", kind = "framework")]
unsafe extern "C" {}

pub(crate) fn status() -> ContextMenuStatus {
    use objc2_app_kit::NSWorkspace;
    use objc2_foundation::NSString;
    let bundled = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent()?.parent().map(Path::to_path_buf))
        .is_some_and(|contents| contents.join("PlugIns/MToolFinder.appex").is_dir());
    let enabled: bool =
        unsafe { objc2::msg_send![objc2::class!(FIFinderSyncController), isExtensionEnabled] };
    let vscode_installed = NSWorkspace::sharedWorkspace()
        .URLForApplicationWithBundleIdentifier(&NSString::from_str("com.microsoft.VSCode"))
        .is_some();
    ContextMenuStatus {
        supported: true,
        bundled,
        enabled,
        vscode_installed,
        accessibility_granted: super::finder::accessibility_granted(),
        error: None,
    }
}

struct ContextJob(Arc<AtomicUsize>);
impl Drop for ContextJob {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

pub fn open_urls(app: &AppHandle, urls: Vec<tauri::Url>) {
    for url in urls {
        if url.scheme() != "mtool" || url.host_str() != Some("context") {
            continue;
        }
        let jobs = app.state::<ContextMenuState>().platform.active_jobs.clone();
        if jobs
            .try_update(Ordering::AcqRel, Ordering::Relaxed, |n| {
                (n < 4).then_some(n + 1)
            })
            .is_err()
        {
            report_error(app, AppError::new("busy", "右键操作过于频繁，请稍后重试"));
            continue;
        }
        let job = ContextJob(jobs);
        let app = app.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let _job = job;
            if let Err(error) = perform_action(&app, &url) {
                report_error(&app, error);
            }
        });
    }
}

fn report_error(app: &AppHandle, error: AppError) {
    if let Ok(mut last_error) = lock(&app.state::<ContextMenuState>().error) {
        *last_error = Some(error.clone());
    }
    let _ = crate::common::windowing::show_main(app);
    let _ = app.emit_to("main", "context-menu-error", error.message);
}

fn perform_action(app: &AppHandle, url: &tauri::Url) -> Result<()> {
    let (action, paths) = parse_url(url)?;
    *lock(&app.state::<ContextMenuState>().error)? = None;
    match action {
        "new-file" => {
            create_in_finder(app, &paths[0])?;
        }
        "vscode" => {
            let status = std::process::Command::new("/usr/bin/open")
                .args(["-b", "com.microsoft.VSCode", "--"])
                .args(paths)
                .output()
                .map_err(|_| AppError::new("vscode_unavailable", "无法启动 VS Code"))?;
            if !status.status.success() {
                return Err(AppError::new(
                    "vscode_unavailable",
                    "无法打开 VS Code，请确认已安装",
                ));
            }
        }
        _ => unreachable!("parse_url only accepts known actions"),
    }
    Ok(())
}

fn parse_url(url: &tauri::Url) -> Result<(&str, Vec<PathBuf>)> {
    if url.as_str().len() > 64 * 1024
        || url.scheme() != "mtool"
        || url.host_str() != Some("context")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some()
        || url.fragment().is_some()
    {
        return Err(AppError::invalid("右键菜单请求无效"));
    }
    if !matches!(url.path(), "/new-file" | "/vscode") {
        return Err(AppError::invalid("右键菜单操作无效"));
    }
    let pairs = url.query_pairs().collect::<Vec<_>>();
    if pairs.is_empty() || pairs.len() > 32 {
        return Err(AppError::invalid("每次最多选择 32 个文件或文件夹"));
    }
    let paths = pairs
        .into_iter()
        .map(|(key, path)| {
            if key != "path" || path.contains('\0') || !Path::new(path.as_ref()).is_absolute() {
                return Err(AppError::invalid("右键菜单路径无效"));
            }
            std::fs::canonicalize(path.as_ref())
                .map_err(|_| AppError::invalid("目标文件或文件夹不存在"))
        })
        .collect::<Result<Vec<_>>>()?;
    if url.path() == "/new-file" && (paths.len() != 1 || !paths[0].is_dir()) {
        return Err(AppError::invalid("请选择一个文件夹新增文件"));
    }
    Ok((&url.path()[1..], paths))
}
