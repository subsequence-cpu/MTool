use mtool_core::{ToolDefinition, ToolId, ToolOperation};

fn tool(
    id: ToolId,
    title: &str,
    description: &str,
    classification: (&str, &str),
    keywords: &[&str],
    operations: &[(&str, &str)],
    sample: &str,
) -> ToolDefinition {
    ToolDefinition {
        id,
        title: title.into(),
        description: description.into(),
        category: classification.0.into(),
        icon: classification.1.into(),
        keywords: keywords.iter().map(|s| (*s).into()).collect(),
        operations: operations
            .iter()
            .map(|(id, label)| ToolOperation {
                id: (*id).into(),
                label: (*label).into(),
            })
            .collect(),
        sample: sample.into(),
    }
}

pub fn catalog() -> Vec<ToolDefinition> {
    use ToolId::*;
    vec![
        tool(Json, "JSON 工具", "格式化 · 校验 · 压缩 · 转义", ("格式与文本", "braces"), &["json", "格式化", "校验", "转义"],
            &[("format","格式化"),("compact","压缩"),("validate","校验"),("escape","转义"),("unescape","去转义")],
            r#"{"name":"MTool","platforms":["macOS","Windows"],"offline":true}"#),
        tool(Base64,"Base64","编码 · 解码",("编解码", "binary"), &["base64", "编码", "解码"], &[("encode","编码"),("decode","解码")],"Hello, MTool!"),
        tool(Url,"URL 编解码","编码 · 解码",("编解码", "link"), &["url","percent","百分号"], &[("encode","编码"),("decode","解码")],"https://example.com/search?q=你好 世界"),
        tool(Unicode,"Unicode 转换","转义 · 还原",("编解码", "languages"), &["unicode","中文","转义"], &[("encode","转义"),("decode","还原")],"你好，MTool 👋"),
        tool(Hex,"Hex / String","文本 ↔ Hex",("编解码", "code"), &["hex","十六进制","string"], &[("encode","文本转 Hex"),("decode","Hex 转文本")],"Hello, 世界"),
        tool(Timestamp,"时间戳转换","时间戳 ↔ 日期",("生成与转换", "clock"), &["timestamp","时间","日期","毫秒"], &[("from-timestamp","时间戳 → 日期"),("to-timestamp","日期 → 时间戳")],"1792112400"),
        tool(Hash,"哈希计算","MD5 · SHA · HMAC",("加密与校验", "fingerprint"), &["hash","sha","md5","hmac"], &[("digest","哈希"),("hmac","HMAC")],"Hello, MTool!"),
        tool(Jwt,"JWT 解码","Header · Payload（未验签）",("加密与校验", "key"), &["jwt","token","令牌"], &[("decode","解码")],"eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJtdG9vbCIsIm5hbWUiOiJN VG9vbCJ9.signature"),
        tool(Uuid,"UUID / 随机字符","UUID v4 · 随机字符串",("生成与转换", "shuffle"), &["uuid","random","随机"], &[("uuid","UUID v4"),("random","随机字符串")],""),
        tool(Text,"文本处理","去重 · 排序 · 大小写 · 统计",("格式与文本", "text"), &["text","文本","去重","统计"], &[("dedupe","按行去重"),("sort","按行排序"),("upper","大写"),("lower","小写"),("trim","清理空白"),("stats","统计")],"Rust\nTypeScript\nRust\nVue\n"),
        tool(Diff,"文本对比","逐行对比",("格式与文本", "diff"), &["diff","对比","差异"], &[("compare","对比")],"name: MTool\nversion: 0.1.0\noffline: true\n"),
        tool(Regex,"正则表达式","匹配 · 捕获 · 替换",("加密与校验", "regex"), &["regex","正则","匹配","替换"], &[("match","匹配"),("replace","替换")],"contact: hello@example.com\nwebsite: example.com"),
    ]
}
