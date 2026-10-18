mod crypto;
mod datetime;
mod encoding;
mod json;
mod registry;
mod text;

use mtool_core::{
    AppError, Result, ToolId, ToolRequest, ToolResponse, MAX_OUTPUT_BYTES, MAX_TEXT_BYTES,
};

pub use registry::catalog;

pub fn execute(request: &ToolRequest) -> Result<ToolResponse> {
    if request.input.len() > MAX_TEXT_BYTES
        || request.secondary.len() > MAX_TEXT_BYTES
        || request.options.len() > 16
        || request.options.values().any(|v| v.len() > 8192)
    {
        return Err(AppError::invalid(
            "输入过大，请减少内容后重试（普通文本上限 2 MiB）",
        ));
    }
    let definition = catalog()
        .into_iter()
        .find(|t| t.id == request.tool)
        .expect("all tool IDs registered");
    if !definition
        .operations
        .iter()
        .any(|op| op.id == request.operation)
    {
        return Err(AppError::invalid("此工具不支持该操作"));
    }
    let response = match request.tool {
        ToolId::Json => json::execute(request),
        ToolId::Base64 | ToolId::Url | ToolId::Unicode | ToolId::Hex => encoding::execute(request),
        ToolId::Timestamp => datetime::execute(request),
        ToolId::Hash | ToolId::Jwt | ToolId::Uuid => crypto::execute(request),
        ToolId::Text | ToolId::Diff | ToolId::Regex => text::execute(request),
    }?;
    if response.output.len() > MAX_OUTPUT_BYTES {
        return Err(AppError::invalid("输出超过 8 MiB，请减少输入或匹配数量"));
    }
    Ok(response)
}
