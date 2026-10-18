use mtool_core::{AppError, Result, ToolId, ToolRequest, ToolResponse, MAX_OUTPUT_BYTES};
use regex::RegexBuilder;
use similar::{ChangeTag, TextDiff};
use std::{
    collections::HashSet,
    time::{Duration, Instant},
};
use unicode_segmentation::UnicodeSegmentation;

pub fn execute(request: &ToolRequest) -> Result<ToolResponse> {
    let output=match request.tool {
        ToolId::Diff=>{
            if request.input.len()+request.secondary.len()>256*1024 || request.input.lines().count()+request.secondary.lines().count()>10_000 {
                return Err(AppError::invalid("文本对比总输入上限为 256 KiB、10,000 行"));
            }
            let diff=TextDiff::configure().deadline(Instant::now()+Duration::from_secs(2)).diff_lines(&request.input,&request.secondary);
            let mut output=String::new();
            for change in diff.iter_all_changes(){output.push(match change.tag(){ChangeTag::Delete=>'-',ChangeTag::Insert=>'+',ChangeTag::Equal=>' '});output.push_str(change.value());if change.missing_newline(){output.push('\n');}}
            output
        }
        ToolId::Regex=>{
            let pattern=request.option("pattern","");
            if pattern.is_empty() || pattern.len()>4096{return Err(AppError::invalid("正则表达式需为 1–4096 字节"));}
            let regex=RegexBuilder::new(pattern).size_limit(2*1024*1024).dfa_size_limit(2*1024*1024).build()
                .map_err(|_|AppError::invalid("正则无效或过于复杂。使用 Rust regex 语法，不支持回溯引用和前后查找。"))?;
            if request.operation=="replace" {
                let replacement=request.option("replacement","");
                let mut output=String::new();let mut last=0;
                for (index,captures) in regex.captures_iter(&request.input).enumerate(){
                    if index>=10_000{return Err(AppError::invalid("替换匹配数超过 10,000"));}
                    let whole=captures.get(0).expect("regex whole match");output.push_str(&request.input[last..whole.start()]);captures.expand(replacement,&mut output);last=whole.end();
                    if output.len()>MAX_OUTPUT_BYTES{return Err(AppError::invalid("替换输出过大"));}
                }
                output.push_str(&request.input[last..]);output
            } else {
                let mut matches=Vec::new();let mut captured_bytes=0usize;
                for (index,captures) in regex.captures_iter(&request.input).enumerate(){
                    if index>=1000{return Err(AppError::invalid("匹配数超过 1,000，请缩小输入范围"));}
                    let whole=captures.get(0).expect("regex whole match");
                    captured_bytes=captured_bytes.saturating_add(captures.iter().flatten().map(|m|m.len()).sum::<usize>());
                    if captured_bytes>MAX_OUTPUT_BYTES{return Err(AppError::invalid("捕获内容总量超过 8 MiB，请减少捕获组或输入"));}
                    matches.push(serde_json::json!({"match":whole.as_str(),"startByte":whole.start(),"endByte":whole.end(),"groups":captures.iter().skip(1).map(|m|m.map(|v|v.as_str())).collect::<Vec<_>>()}));
                }
                serde_json::to_string_pretty(&matches).map_err(|_|AppError::invalid("匹配结果无法序列化"))?
            }
        }
        _=> match request.operation.as_str(){
            "dedupe"=>{let mut seen=HashSet::new();request.input.lines().filter(|line|seen.insert(*line)).collect::<Vec<_>>().join("\n")},
            "sort"=>{let mut lines=request.input.lines().collect::<Vec<_>>();lines.sort_unstable();lines.join("\n")},
            "upper"=>request.input.to_uppercase(),"lower"=>request.input.to_lowercase(),
            "trim"=>request.input.lines().map(str::trim).filter(|l|!l.is_empty()).collect::<Vec<_>>().join("\n"),
            "stats"=>format!("字符（字素簇）  {}\nUnicode 标量    {}\nUTF-8 字节      {}\n行数            {}\n空白分隔词数    {}",request.input.graphemes(true).count(),request.input.chars().count(),request.input.len(),request.input.lines().count(),request.input.split_whitespace().count()),
            _=>return Err(AppError::invalid("不支持的文本操作")),
        }
    };
    Ok(ToolResponse::text(output))
}
