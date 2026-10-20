use mtool_core::{AppError, AppStatus, ClipboardDetail, Result, Settings};
use mtool_storage::Store;
use std::{
    path::Path,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex, MutexGuard,
    },
};

pub struct Persistence {
    pub store: Option<Store>,
    pub settings: Settings,
    pub error: Option<String>,
}

pub struct AppState {
    pub persistence: Mutex<Persistence>,
    pub clipboard_io: Mutex<()>,
    pub own_revision: Mutex<Option<u64>>,
    pub capture_error: Mutex<Option<String>>,
    pub shortcut_error: Mutex<Option<String>>,
    pub clipboard_focused: AtomicBool,
    #[cfg(not(target_os = "macos"))]
    pub clipboard_positioned: AtomicBool,
    pub preview: Mutex<Option<ClipboardDetail>>,
    pub preview_generation: AtomicUsize,
    pub preview_ready: AtomicBool,
    pub stop: AtomicBool,
    pub active_jobs: AtomicUsize,
}

pub fn lock<T>(mutex: &Mutex<T>) -> Result<MutexGuard<'_, T>> {
    mutex
        .lock()
        .map_err(|_| AppError::new("state_unavailable", "应用状态暂时不可用，请重新启动"))
}

impl AppState {
    pub fn initialize(directory: &Path) -> Self {
        let persistence = (|| -> Result<Persistence> {
            std::fs::create_dir_all(directory)
                .map_err(|_| AppError::new("data_directory_unavailable", "无法创建应用数据目录"))?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(directory, std::fs::Permissions::from_mode(0o700))
                    .map_err(|_| {
                        AppError::new("data_directory_unavailable", "无法保护应用数据目录权限")
                    })?;
            }
            let path = directory.join("mtool.db");
            let database_exists = path
                .try_exists()
                .map_err(|_| AppError::new("data_directory_unavailable", "无法访问历史数据库"))?;
            let key = mtool_platform::database_key(directory, database_exists)?;
            let store = Store::open(&path, &key)?;
            let settings = store.settings()?;
            store.prune(&settings, chrono::Utc::now().timestamp_millis())?;
            Ok(Persistence {
                store: Some(store),
                settings,
                error: None,
            })
        })()
        .unwrap_or_else(|error| Persistence {
            store: None,
            settings: Settings::default(),
            error: Some(error.message),
        });
        Self {
            persistence: Mutex::new(persistence),
            clipboard_io: Mutex::new(()),
            own_revision: Mutex::new(None),
            capture_error: Mutex::new(None),
            shortcut_error: Mutex::new(None),
            clipboard_focused: AtomicBool::new(false),
            #[cfg(not(target_os = "macos"))]
            clipboard_positioned: AtomicBool::new(false),
            preview: Mutex::new(None),
            preview_generation: AtomicUsize::new(0),
            preview_ready: AtomicBool::new(false),
            stop: AtomicBool::new(false),
            active_jobs: AtomicUsize::new(0),
        }
    }

    pub fn settings(&self) -> Result<Settings> {
        Ok(lock(&self.persistence)?.settings.clone())
    }
    pub fn status(&self) -> Result<AppStatus> {
        let p = lock(&self.persistence)?;
        Ok(AppStatus {
            platform: std::env::consts::OS.into(),
            storage_ready: p.store.is_some(),
            storage_error: p.error.clone(),
            capture_enabled: p.store.is_some() && p.settings.capture_enabled,
            capture_error: lock(&self.capture_error)?.clone(),
            shortcut_error: lock(&self.shortcut_error)?.clone(),
            version: env!("CARGO_PKG_VERSION").into(),
        })
    }

    pub fn job(self: &Arc<Self>) -> Result<JobGuard> {
        self.active_jobs
            .try_update(Ordering::AcqRel, Ordering::Relaxed, |n| {
                if n < 2 {
                    Some(n + 1)
                } else {
                    None
                }
            })
            .map_err(|_| AppError::new("busy", "已有两个工具任务运行中，请稍后重试"))?;
        Ok(JobGuard(self.clone()))
    }
}

pub struct JobGuard(Arc<AppState>);
impl Drop for JobGuard {
    fn drop(&mut self) {
        self.0.active_jobs.fetch_sub(1, Ordering::AcqRel);
    }
}

pub fn unavailable() -> AppError {
    AppError::new(
        "storage_unavailable",
        "加密历史不可用，请查看设置中的存储状态；工具箱仍可使用",
    )
}
