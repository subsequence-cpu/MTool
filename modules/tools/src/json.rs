use mtool_core::{AppError, Result, ToolRequest, ToolResponse};

fn parse(input: &str) -> Result<serde_json::Value> {
    serde_json::from_str(input).map_err(|e| {
        AppError::invalid(format!(
            "JSON 无效：第 {} 行、第 {} 列（请检查语法与嵌套深度）",
            e.line(),
            e.column()
        ))
    })
}

pub fn execute(request: &ToolRequest) -> Result<ToolResponse> {
    let output = match request.operation.as_str() {
        "escape" => {
            serde_json::to_string(&request.input).map_err(|_| AppError::invalid("无法转义输入"))?
        }
        "unescape" => serde_json::from_str::<String>(&request.input)
            .map_err(|_| AppError::invalid("请输入带双引号的 JSON 字符串"))?,
        "validate" => {
            parse(&request.input)?;
            "JSON 有效。\n数字保留原始精度，键顺序保持不变。".into()
        }
        "compact" => serde_json::to_string(&parse(&request.input)?)
            .map_err(|_| AppError::invalid("无法序列化 JSON"))?,
        _ => serde_json::to_string_pretty(&parse(&request.input)?)
            .map_err(|_| AppError::invalid("无法序列化 JSON"))?,
    };
    Ok(ToolResponse::text(output))
}
