use std::{fs, path::Path};

fn main() -> std::io::Result<()> {
    let catalog =
        serde_json::to_string_pretty(&mtool_tools::catalog()).expect("catalog serializes");
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../ui/bindings/catalog.ts");
    fs::create_dir_all(path.parent().expect("bindings has parent"))?;
    fs::write(path,format!("import type {{ ToolDefinition }} from './types';\n\nexport const catalog: ToolDefinition[] = {catalog};\n"))
}
