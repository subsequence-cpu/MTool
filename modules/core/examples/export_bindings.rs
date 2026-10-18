use mtool_core::*;
use std::{fs, path::Path};
use ts_rs::TS;

fn main() -> std::io::Result<()> {
    let declarations = [
        AppError::decl(),
        ContextMenuStatus::decl(),
        ToolId::decl(),
        ToolOperation::decl(),
        ToolDefinition::decl(),
        ToolRequest::decl(),
        ToolResponse::decl(),
        ClipboardKind::decl(),
        ClipboardItem::decl(),
        ClipboardDetail::decl(),
        Settings::decl(),
        AppStatus::decl(),
        Bootstrap::decl(),
        ClipboardPage::decl(),
    ];
    let defaults =
        serde_json::to_string_pretty(&Settings::default()).expect("default settings serialize");
    let output = format!(
        "{}\n\nexport const defaultSettings: Settings = {defaults};\n",
        declarations
            .into_iter()
            .map(|d| format!("export {d}"))
            .collect::<Vec<_>>()
            .join("\n\n")
    );
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../ui/bindings/types.ts");
    fs::create_dir_all(path.parent().expect("bindings has parent"))?;
    fs::write(path, output)
}
