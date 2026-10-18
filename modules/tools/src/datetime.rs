use chrono::{DateTime, Local, Utc};
use mtool_core::{AppError, Result, ToolRequest, ToolResponse};

pub fn execute(request: &ToolRequest) -> Result<ToolResponse> {
    let date = if request.operation == "to-timestamp" {
        DateTime::parse_from_rfc3339(request.input.trim())
            .map_err(|_| {
                AppError::invalid("请输入包含时区的 RFC 3339 日期，例如 2026-10-16T09:00:00+08:00")
            })?
            .with_timezone(&Utc)
    } else {
        let number = request
            .input
            .trim()
            .parse::<i64>()
            .map_err(|_| AppError::invalid("请输入整数时间戳"))?;
        match request.option("unit", "seconds") {
            "seconds" => DateTime::from_timestamp(number, 0),
            "milliseconds" => DateTime::from_timestamp_millis(number),
            _ => return Err(AppError::invalid("时间单位必须是秒或毫秒")),
        }
        .ok_or_else(|| AppError::invalid("时间戳超出有效范围"))?
    };
    Ok(ToolResponse::text(format!(
        "UTC      {}\n本地时间  {}\n\n秒        {}\n毫秒      {}",
        date.to_rfc3339(),
        date.with_timezone(&Local).to_rfc3339(),
        date.timestamp(),
        date.timestamp_millis()
    )))
}
