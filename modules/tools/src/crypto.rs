use base64::{
    engine::general_purpose::{URL_SAFE, URL_SAFE_NO_PAD},
    Engine,
};
use hmac::{Hmac, Mac};
use md5::Md5;
use mtool_core::{AppError, Result, ToolId, ToolRequest, ToolResponse};
use sha1::Sha1;
use sha2::{Digest, Sha256, Sha512};

pub fn execute(request: &ToolRequest) -> Result<ToolResponse> {
    match request.tool {
        ToolId::Hash => hash(request),
        ToolId::Jwt => jwt(request),
        ToolId::Uuid => random(request),
        _ => Err(AppError::invalid("不支持的加密工具")),
    }
}

fn hash(request: &ToolRequest) -> Result<ToolResponse> {
    let bytes = request.input.as_bytes();
    let algorithm = request.option("algorithm", "sha256");
    let output = if request.operation == "hmac" {
        let key = request.option("key", "").as_bytes();
        if key.is_empty() {
            return Err(AppError::invalid("请输入 HMAC 密钥"));
        }
        macro_rules! mac {
            ($hash:ty) => {{
                let mut mac = Hmac::<$hash>::new_from_slice(key)
                    .map_err(|_| AppError::invalid("HMAC 密钥无效"))?;
                mac.update(bytes);
                hex::encode(mac.finalize().into_bytes())
            }};
        }
        match algorithm {
            "md5" => mac!(Md5),
            "sha1" => mac!(Sha1),
            "sha256" => mac!(Sha256),
            "sha512" => mac!(Sha512),
            _ => return Err(AppError::invalid("不支持的哈希算法")),
        }
    } else {
        match algorithm {
            "md5" => hex::encode(Md5::digest(bytes)),
            "sha1" => hex::encode(Sha1::digest(bytes)),
            "sha256" => hex::encode(Sha256::digest(bytes)),
            "sha512" => hex::encode(Sha512::digest(bytes)),
            _ => return Err(AppError::invalid("不支持的哈希算法")),
        }
    };
    let mut response = ToolResponse::text(output);
    if matches!(algorithm, "md5" | "sha1") {
        response.notice = Some("MD5 / SHA-1 仅用于兼容和校验，不适用于安全签名或密码存储。".into());
    }
    Ok(response)
}

fn jwt(request: &ToolRequest) -> Result<ToolResponse> {
    let parts: Vec<_> = request.input.trim().split('.').collect();
    if parts.len() != 3 {
        return Err(AppError::invalid("JWT 应由三个以点分隔的部分组成"));
    }
    let decode = |part: &str| -> Result<serde_json::Value> {
        let bytes = URL_SAFE_NO_PAD
            .decode(part)
            .or_else(|_| URL_SAFE.decode(part))
            .map_err(|_| AppError::invalid("JWT 包含无效的 Base64URL 内容"))?;
        serde_json::from_slice(&bytes)
            .map_err(|_| AppError::invalid("JWT Header 或 Payload 不是有效 JSON"))
    };
    let output = serde_json::to_string_pretty(
        &serde_json::json!({"header":decode(parts[0])?,"payload":decode(parts[1])?}),
    )
    .map_err(|_| AppError::invalid("JWT 无法序列化"))?;
    Ok(ToolResponse {
        output,
        notice: Some("仅解码，未验证签名、有效期、签发方或受众。".into()),
    })
}

fn random(request: &ToolRequest) -> Result<ToolResponse> {
    let count = request
        .option("count", "1")
        .parse::<usize>()
        .map_err(|_| AppError::invalid("数量应为整数"))?;
    if !(1..=100).contains(&count) {
        return Err(AppError::invalid("每次生成数量需为 1–100"));
    }
    if request.operation == "uuid" {
        return Ok(ToolResponse::text(
            (0..count)
                .map(|_| uuid::Uuid::new_v4().to_string())
                .collect::<Vec<_>>()
                .join("\n"),
        ));
    }
    let length = request
        .option("length", "32")
        .parse::<usize>()
        .map_err(|_| AppError::invalid("长度应为整数"))?;
    if !(1..=1024).contains(&length) {
        return Err(AppError::invalid("字符串长度需为 1–1024"));
    }
    const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut output = Vec::with_capacity(count);
    for _ in 0..count {
        let mut value = String::with_capacity(length);
        while value.len() < length {
            let mut bytes = [0u8; 128];
            getrandom::fill(&mut bytes)
                .map_err(|_| AppError::new("random_unavailable", "系统随机数不可用"))?;
            for byte in bytes {
                if byte < 248 {
                    value.push(ALPHABET[usize::from(byte) % 62] as char);
                    if value.len() == length {
                        break;
                    }
                }
            }
        }
        output.push(value);
    }
    Ok(ToolResponse::text(output.join("\n")))
}
