use std::{
    fs::{self, File},
    io::{ErrorKind, Read, Write},
    path::Path,
};

use mtool_core::{AppError, Result};
use zeroize::Zeroizing;

const KEY_FILE: &str = "mtool.key";

pub fn database_key(directory: &Path, database_exists: bool) -> Result<Zeroizing<[u8; 32]>> {
    if let Some(key) = read_key(directory)? {
        return Ok(key);
    }
    if database_exists {
        return Err(AppError::new(
            "key_file_missing",
            "本地密钥缺失，请从备份恢复 mtool.key",
        ));
    }
    let mut key = Zeroizing::new([0; 32]);
    getrandom::fill(key.as_mut())
        .map_err(|_| AppError::new("random_unavailable", "无法生成数据库密钥"))?;
    persist_key(directory, &key)
}

fn key_file_error() -> AppError {
    AppError::new(
        "key_file_unavailable",
        "无法访问本地密钥，请检查数据目录权限",
    )
}

fn invalid_key() -> AppError {
    AppError::new("key_file_invalid", "本地密钥无效，请从备份恢复 mtool.key")
}

fn read_key(directory: &Path) -> Result<Option<Zeroizing<[u8; 32]>>> {
    let path = directory.join(KEY_FILE);
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(key_file_error()),
    };
    if !metadata.is_file() || metadata.len() != 32 {
        return Err(invalid_key());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
            .map_err(|_| key_file_error())?;
    }
    let mut file = File::open(&path).map_err(|_| key_file_error())?;
    let mut key = Zeroizing::new([0; 32]);
    file.read_exact(key.as_mut()).map_err(|_| invalid_key())?;
    let mut extra = Zeroizing::new([0; 1]);
    if file.read(extra.as_mut()).map_err(|_| key_file_error())? != 0 {
        return Err(invalid_key());
    }
    Ok(Some(key))
}

fn persist_key(directory: &Path, key: &[u8; 32]) -> Result<Zeroizing<[u8; 32]>> {
    let mut builder = tempfile::Builder::new();
    builder.prefix(".mtool-key-");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        builder.permissions(fs::Permissions::from_mode(0o600));
    }
    let mut temporary = builder
        .tempfile_in(directory)
        .map_err(|_| key_file_error())?;
    temporary.write_all(key).map_err(|_| key_file_error())?;
    temporary
        .as_file()
        .sync_all()
        .map_err(|_| key_file_error())?;
    match temporary.persist_noclobber(directory.join(KEY_FILE)) {
        Ok(_) => {}
        Err(error) if error.error.kind() == ErrorKind::AlreadyExists => {
            drop(error);
        }
        Err(_) => return Err(key_file_error()),
    }
    #[cfg(unix)]
    File::open(directory)
        .and_then(|directory| directory.sync_all())
        .map_err(|_| key_file_error())?;
    read_key(directory)?.ok_or_else(key_file_error)
}
