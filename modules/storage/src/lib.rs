use std::path::Path;

use base64::{engine::general_purpose::STANDARD, Engine};
use mtool_core::{
    AppError, ClipboardContent, ClipboardDetail, ClipboardItem, ClipboardKind, ClipboardPage,
    Result, Settings,
};
use rusqlite::{params, Connection, OptionalExtension};
use zeroize::Zeroizing;

fn database_error(_: rusqlite::Error) -> AppError {
    AppError::new(
        "database_error",
        "加密数据库操作失败，请检查磁盘空间与文件访问权限",
    )
}

pub struct Store {
    connection: Connection,
}

impl Store {
    pub fn open(path: &Path, key: &[u8; 32]) -> Result<Self> {
        let connection = Connection::open(path).map_err(database_error)?;
        let key_hex = Zeroizing::new(format!("x'{}'", hex::encode(key)));
        connection
            .pragma_update(None, "key", key_hex.as_str())
            .map_err(database_error)?;
        let cipher: String = connection
            .pragma_query_value(None, "cipher_version", |r| r.get(0))
            .map_err(database_error)?;
        if cipher.is_empty() {
            return Err(AppError::new(
                "encryption_unavailable",
                "当前构建未启用 SQLCipher，已停止持久化",
            ));
        }
        connection
            .query_row("SELECT count(*) FROM sqlite_master", [], |r| {
                r.get::<_, i64>(0)
            })
            .map_err(|_| AppError::new("database_locked", "无法解锁历史数据库，请检查密钥备份"))?;
        connection
            .busy_timeout(std::time::Duration::from_secs(3))
            .map_err(database_error)?;
        connection
            .execute_batch(
                "PRAGMA temp_store=MEMORY; PRAGMA secure_delete=ON; PRAGMA journal_mode=WAL;",
            )
            .map_err(database_error)?;
        let version: i64 = connection
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .map_err(database_error)?;
        if version > 1 {
            return Err(AppError::new(
                "database_newer",
                "数据库来自较新版本，请升级 MTool 后再打开",
            ));
        }
        if version == 0 {
            connection
                .execute_batch(include_str!("../migrations/001_initial.sql"))
                .map_err(database_error)?;
        }
        Ok(Self { connection })
    }

    pub fn settings(&self) -> Result<Settings> {
        let json: Option<String> = self
            .connection
            .query_row(
                "SELECT value FROM settings WHERE key='preferences'",
                [],
                |r| r.get(0),
            )
            .optional()
            .map_err(database_error)?;
        let settings: Settings = match json {
            Some(json) => serde_json::from_str(&json).map_err(|_| {
                AppError::new("settings_invalid", "保存的设置无法读取，请检查设置数据")
            })?,
            None => Settings::default(),
        };
        settings.validate()?;
        Ok(settings)
    }

    pub fn save_settings(&mut self, settings: &Settings, now: i64) -> Result<()> {
        settings.validate()?;
        let pinned_bytes: i64 = self
            .connection
            .query_row(
                "SELECT COALESCE(SUM(byte_size),0) FROM history WHERE pinned=1",
                [],
                |r| r.get(0),
            )
            .map_err(database_error)?;
        if pinned_bytes > i64::from(settings.max_storage_mb) * 1024 * 1024 {
            return Err(AppError::invalid(
                "收藏内容已超过新容量上限，请先删除部分收藏",
            ));
        }
        let json = serde_json::to_string(settings)
            .map_err(|_| AppError::new("settings_invalid", "设置无法序列化"))?;
        let transaction = self.connection.transaction().map_err(database_error)?;
        transaction.execute("INSERT INTO settings(key,value) VALUES ('preferences',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value",[json]).map_err(database_error)?;
        prune(&transaction, settings, now)?;
        transaction.commit().map_err(database_error)
    }

    pub fn insert(
        &mut self,
        id: &str,
        content: &ClipboardContent,
        source: Option<&str>,
        settings: &Settings,
        now: i64,
    ) -> Result<bool> {
        let (kind, preview, search, width, height) = match content {
            ClipboardContent::Text(text) => (
                "text",
                text.chars().take(180).collect::<String>(),
                text.as_str(),
                None,
                None,
            ),
            ClipboardContent::Image { width, height, .. } => (
                "image",
                format!("图片 · {width} × {height}"),
                "",
                Some(*width),
                Some(*height),
            ),
        };
        let transaction = self.connection.transaction().map_err(database_error)?;
        transaction.execute(
            "INSERT INTO history(id,kind,preview,search_text,payload,source_app,created_at,updated_at,pinned,byte_size,width,height) VALUES(?1,?2,?3,?4,?5,?6,?7,?7,0,?8,?9,?10) ON CONFLICT(id) DO UPDATE SET updated_at=excluded.updated_at,source_app=excluded.source_app",
            params![id,kind,preview,search,content.bytes(),source,now,content.bytes().len() as u32,width,height],
        ).map_err(database_error)?;
        prune(&transaction, settings, now)?;
        let exists = transaction
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM history WHERE id=?1)",
                [id],
                |r| r.get(0),
            )
            .map_err(database_error)?;
        transaction.commit().map_err(database_error)?;
        Ok(exists)
    }

    pub fn list(
        &self,
        query: &str,
        pinned_only: bool,
        offset: u32,
        limit: u32,
    ) -> Result<ClipboardPage> {
        if query.len() > 512 || offset > 20_000 {
            return Err(AppError::invalid("搜索条件或分页范围过大"));
        }
        let limit = limit.clamp(1, 100);
        let pattern = format!(
            "%{}%",
            query
                .replace('\\', "\\\\")
                .replace('%', "\\%")
                .replace('_', "\\_")
        );
        let condition="(?1=0 OR pinned=1) AND (search_text LIKE ?2 ESCAPE '\\' OR preview LIKE ?2 ESCAPE '\\' OR source_app LIKE ?2 ESCAPE '\\')";
        let total: u32 = self
            .connection
            .query_row(
                &format!("SELECT count(*) FROM history WHERE {condition}"),
                params![pinned_only, pattern],
                |r| r.get(0),
            )
            .map_err(database_error)?;
        let mut statement=self.connection.prepare(&format!("SELECT id,kind,preview,source_app,created_at,updated_at,pinned,byte_size,width,height FROM history WHERE {condition} ORDER BY pinned DESC,updated_at DESC,id ASC LIMIT ?3 OFFSET ?4")).map_err(database_error)?;
        let items = statement
            .query_map(params![pinned_only, pattern, limit, offset], row_item)
            .map_err(database_error)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(database_error)?;
        let has_more = offset.saturating_add(items.len() as u32) < total;
        Ok(ClipboardPage {
            items,
            total,
            has_more,
        })
    }

    pub fn content(&self, id: &str) -> Result<(ClipboardItem, ClipboardContent)> {
        let record=self.connection.query_row("SELECT id,kind,preview,source_app,created_at,updated_at,pinned,byte_size,width,height,payload FROM history WHERE id=?1",[id],|row|Ok((row_item(row)?,row.get::<_,Vec<u8>>(10)?))).optional().map_err(database_error)?
            .ok_or_else(||AppError::new("not_found","这条历史已删除或过期"))?;
        let (item, bytes) = record;
        let content = match item.kind {
            ClipboardKind::Text => ClipboardContent::Text(
                String::from_utf8(bytes)
                    .map_err(|_| AppError::new("invalid_history", "历史文本编码无效"))?,
            ),
            ClipboardKind::Image => ClipboardContent::Image {
                png: bytes,
                width: item.width.unwrap_or(0),
                height: item.height.unwrap_or(0),
            },
        };
        Ok((item, content))
    }

    pub fn detail(&self, id: &str) -> Result<ClipboardDetail> {
        let (item, content) = self.content(id)?;
        Ok(match content {
            ClipboardContent::Text(text) => ClipboardDetail {
                item,
                text: Some(text),
                image_data_url: None,
            },
            ClipboardContent::Image { png, .. } => ClipboardDetail {
                item,
                text: None,
                image_data_url: Some(format!("data:image/png;base64,{}", STANDARD.encode(png))),
            },
        })
    }

    pub fn pin(&mut self, id: &str, pinned: bool, settings: &Settings, now: i64) -> Result<()> {
        let transaction = self.connection.transaction().map_err(database_error)?;
        let changed = transaction
            .execute(
                "UPDATE history SET pinned=?1 WHERE id=?2",
                params![pinned, id],
            )
            .map_err(database_error)?;
        if changed == 0 {
            return Err(AppError::new("not_found", "这条历史已删除或过期"));
        }
        prune(&transaction, settings, now)?;
        transaction.commit().map_err(database_error)
    }

    pub fn delete(&self, id: &str) -> Result<()> {
        self.connection
            .execute("DELETE FROM history WHERE id=?1", [id])
            .map_err(database_error)?;
        Ok(())
    }

    pub fn clear(&self, include_pinned: bool) -> Result<()> {
        self.connection
            .execute(
                "DELETE FROM history WHERE ?1=1 OR pinned=0",
                [include_pinned],
            )
            .map_err(database_error)?;
        Ok(())
    }

    pub fn prune(&self, settings: &Settings, now: i64) -> Result<()> {
        prune(&self.connection, settings, now)
    }
}

fn row_item(row: &rusqlite::Row<'_>) -> rusqlite::Result<ClipboardItem> {
    let kind: String = row.get(1)?;
    Ok(ClipboardItem {
        id: row.get(0)?,
        kind: if kind == "image" {
            ClipboardKind::Image
        } else {
            ClipboardKind::Text
        },
        preview: row.get(2)?,
        source_app: row.get(3)?,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
        pinned: row.get(6)?,
        byte_size: row.get(7)?,
        width: row.get(8)?,
        height: row.get(9)?,
    })
}

fn prune(connection: &Connection, settings: &Settings, now: i64) -> Result<()> {
    let cutoff = now.saturating_sub(i64::from(settings.retention_days) * 86_400_000);
    connection
        .execute(
            "DELETE FROM history WHERE pinned=0 AND updated_at<?1",
            [cutoff],
        )
        .map_err(database_error)?;
    connection.execute("DELETE FROM history WHERE id IN (SELECT id FROM history WHERE pinned=0 ORDER BY updated_at DESC,id ASC LIMIT -1 OFFSET ?1)",[settings.history_limit]).map_err(database_error)?;
    let mut size: i64 = connection
        .query_row("SELECT COALESCE(SUM(byte_size),0) FROM history", [], |r| {
            r.get(0)
        })
        .map_err(database_error)?;
    let capacity = i64::from(settings.max_storage_mb) * 1024 * 1024;
    while size > capacity {
        let oldest:Option<(String,i64)>=connection.query_row("SELECT id,byte_size FROM history WHERE pinned=0 ORDER BY updated_at ASC,id DESC LIMIT 1",[],|r|Ok((r.get(0)?,r.get(1)?))).optional().map_err(database_error)?;
        match oldest {
            Some((id, bytes)) => {
                connection
                    .execute("DELETE FROM history WHERE id=?1", [id])
                    .map_err(database_error)?;
                size -= bytes;
            }
            None => break,
        }
    }
    Ok(())
}
