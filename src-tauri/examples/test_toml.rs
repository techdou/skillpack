use std::fs;
use toml_edit::DocumentMut;

fn main() {
    let path = dirs::home_dir().unwrap().join(".codex").join("config.toml");
    let content = fs::read_to_string(&path).unwrap();
    let doc: DocumentMut = content.parse::<DocumentMut>().unwrap();

    if let Some(plugins_table) = doc.get("plugins") {
        if let Some(table) = plugins_table.as_table() {
            println!("Found {} plugins:", table.len());
            for (key, value) in table.iter() {
                if let Some(t) = value.as_table() {
                    let enabled = t.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false);
                    println!("  {} = {}", key, enabled);
                }
            }
        }
    }
}
