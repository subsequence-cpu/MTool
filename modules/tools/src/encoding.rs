use base64::{engine::general_purpose::STANDARD, Engine};
use mtool_core::{AppError, Result, ToolId, ToolRequest, ToolResponse};
use percent_encoding::{percent_decode_str, utf8_percent_encode, NON_ALPHANUMERIC};

pub fn execute(request: &ToolRequest) -> Result<ToolResponse> {
    let encode = request.operation == "encode";
    let output = match request.tool {
        ToolId::Base64 if encode => STANDARD.encode(request.input.as_bytes()),
        ToolId::Base64 => {
            let compact: String = request
                .input
                .chars()
                .filter(|c| !c.is_ascii_whitespace())
                .collect();
            String::from_utf8(
                STANDARD
                    .decode(compact)
                    .map_err(|_| AppError::invalid("Base64 格式无效"))?,
            )
            .map_err(|_| AppError::invalid("解码结果不是 UTF-8 文本"))?
        }
        ToolId::Url if encode => {
            const COMPONENT: &percent_encoding::AsciiSet = &NON_ALPHANUMERIC
                .remove(b'-')
                .remove(b'_')
                .remove(b'.')
                .remove(b'~');
            utf8_percent_encode(&request.input, COMPONENT).to_string()
        }
        ToolId::Url => {
            let bytes = request.input.as_bytes();
            if bytes.iter().enumerate().any(|(i, b)| {
                *b == b'%'
                    && (i + 2 >= bytes.len()
                        || !bytes[i + 1].is_ascii_hexdigit()
                        || !bytes[i + 2].is_ascii_hexdigit())
            }) {
                return Err(AppError::invalid("URL 中包含无效的百分号编码"));
            }
            percent_decode_str(&request.input)
                .decode_utf8()
                .map_err(|_| AppError::invalid("解码结果不是 UTF-8 文本"))?
                .into_owned()
        }
        ToolId::Hex if encode => hex::encode(request.input.as_bytes()),
        ToolId::Hex => {
            let compact: String = request
                .input
                .chars()
                .filter(|c| !c.is_ascii_whitespace())
                .collect();
            String::from_utf8(
                hex::decode(compact)
                    .map_err(|_| AppError::invalid("Hex 需包含偶数个有效十六进制字符"))?,
            )
            .map_err(|_| AppError::invalid("解码结果不是 UTF-8 文本"))?
        }
        ToolId::Unicode if encode => {
            let mut output = String::with_capacity(request.input.len());
            for c in request.input.chars() {
                if c.is_ascii() {
                    match c {
                        '\\' => output.push_str("\\\\"),
                        '"' => output.push_str("\\\""),
                        '\n' => output.push_str("\\n"),
                        '\r' => output.push_str("\\r"),
                        '\t' => output.push_str("\\t"),
                        _ => output.push(c),
                    }
                } else {
                    for code in c.encode_utf16(&mut [0; 2]) {
                        output.push_str(&format!("\\u{code:04x}"));
                    }
                }
            }
            output
        }
        ToolId::Unicode => {
            let mut quoted = String::from("\"");
            let mut escaped = false;
            for c in request.input.chars() {
                match c {
                    '"' if !escaped => quoted.push_str("\\\""),
                    '\n' => quoted.push_str("\\n"),
                    '\r' => quoted.push_str("\\r"),
                    '\t' => quoted.push_str("\\t"),
                    _ => quoted.push(c),
                }
                escaped = c == '\\' && !escaped;
            }
            quoted.push('"');
            serde_json::from_str::<String>(&quoted)
                .map_err(|_| AppError::invalid("Unicode 转义无效，请检查 \\uXXXX 与代理对"))?
        }
        _ => return Err(AppError::invalid("不支持的编码工具")),
    };
    Ok(ToolResponse::text(output))
}
