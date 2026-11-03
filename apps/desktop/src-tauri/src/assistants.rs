//! Panel de asistentes IA: detecta la CLI `harpoon` y si `harpoon mcp` está
//! registrado en Claude Code (`~/.claude.json`) o Codex (`~/.codex/config.toml`),
//! y lo registra o quita sin tocar el resto de esos archivos.

use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::{Value, json};

const SERVER: &str = "harpoon";

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Registration {
    /// Colección a la que apunta (`-C`), si se indica.
    pub collection: Option<String>,
    pub allow_write: bool,
    pub command: String,
}

#[derive(Debug, Serialize)]
pub struct McpStatus {
    /// Ruta de la CLI `harpoon`, si está instalada.
    pub cli: Option<String>,
    pub claude: Option<Registration>,
    pub codex: Option<Registration>,
    pub claude_config: String,
    pub codex_config: String,
}

fn home() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map_or_else(|| PathBuf::from("."), PathBuf::from)
}

fn claude_path() -> PathBuf {
    home().join(".claude.json")
}

fn codex_path() -> PathBuf {
    std::env::var_os("CODEX_HOME")
        .map_or_else(|| home().join(".codex"), PathBuf::from)
        .join("config.toml")
}

/// La CLI en el PATH o donde la deja `cargo install` (la app no siempre hereda el PATH de la terminal).
fn find_cli() -> Option<PathBuf> {
    let exe = if cfg!(windows) {
        "harpoon.exe"
    } else {
        "harpoon"
    };
    let mut dirs: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect())
        .unwrap_or_default();
    dirs.push(home().join(".cargo").join("bin"));
    dirs.into_iter().map(|d| d.join(exe)).find(|p| p.is_file())
}

fn registration(command: &str, args: &[String]) -> Registration {
    Registration {
        collection: args
            .iter()
            .position(|a| a == "-C" || a == "--collection")
            .and_then(|i| args.get(i + 1))
            .cloned(),
        allow_write: args.iter().any(|a| a == "--allow-write"),
        command: command.to_owned(),
    }
}

fn args_for(root: &Path, allow_write: bool) -> Vec<String> {
    let mut args = vec!["mcp".to_owned()];
    if allow_write {
        args.push("--allow-write".into());
    }
    args.extend(["-C".to_owned(), root.to_string_lossy().into_owned()]);
    args
}

fn read_claude(path: &Path) -> Option<Registration> {
    let doc: Value = serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()?;
    let server = doc.get("mcpServers")?.get(SERVER)?;
    let args: Vec<String> = server
        .get("args")
        .and_then(|a| serde_json::from_value(a.clone()).ok())
        .unwrap_or_default();
    Some(registration(server.get("command")?.as_str()?, &args))
}

fn read_codex(path: &Path) -> Option<Registration> {
    let doc: toml_edit::DocumentMut = std::fs::read_to_string(path).ok()?.parse().ok()?;
    let server = doc.get("mcp_servers")?.get(SERVER)?;
    let args: Vec<String> = server
        .get("args")
        .and_then(|a| a.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default();
    Some(registration(server.get("command")?.as_str()?, &args))
}

fn write_claude(path: &Path, entry: Option<(&str, &[String])>) -> Result<(), String> {
    let mut doc: Value = match std::fs::read_to_string(path) {
        Ok(text) => serde_json::from_str(&text)
            .map_err(|e| format!("{} no es JSON válido: {e}", path.display()))?,
        Err(_) => json!({}),
    };
    let root = doc
        .as_object_mut()
        .ok_or("formato inesperado en la configuración de Claude Code")?;
    let servers = root.entry("mcpServers").or_insert_with(|| json!({}));
    let servers = servers
        .as_object_mut()
        .ok_or("`mcpServers` no es un objeto")?;
    match entry {
        Some((command, args)) => {
            servers.insert(
                SERVER.into(),
                json!({"type": "stdio", "command": command, "args": args, "env": {}}),
            );
        }
        None => {
            servers.remove(SERVER);
        }
    }
    let text = serde_json::to_string_pretty(&doc).map_err(|e| e.to_string())?;
    harpoon_core::atomic::write(path, text + "\n").map_err(|e| e.to_string())
}

fn write_codex(path: &Path, entry: Option<(&str, &[String])>) -> Result<(), String> {
    let mut doc: toml_edit::DocumentMut = std::fs::read_to_string(path)
        .unwrap_or_default()
        .parse()
        .map_err(|e| format!("{} no es TOML válido: {e}", path.display()))?;
    if !doc.contains_key("mcp_servers") {
        let mut table = toml_edit::Table::new();
        table.set_implicit(true);
        doc.insert("mcp_servers", toml_edit::Item::Table(table));
    }
    let servers = doc["mcp_servers"]
        .as_table_mut()
        .ok_or("`mcp_servers` no es una tabla")?;
    match entry {
        Some((command, args)) => {
            let mut table = toml_edit::Table::new();
            table.insert("command", toml_edit::value(command));
            let mut list = toml_edit::Array::new();
            for a in args {
                list.push(a.as_str());
            }
            table.insert("args", toml_edit::value(list));
            servers.insert(SERVER, toml_edit::Item::Table(table));
        }
        None => {
            servers.remove(SERVER);
        }
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    harpoon_core::atomic::write(path, doc.to_string()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn mcp_status() -> McpStatus {
    McpStatus {
        cli: find_cli().map(|p| p.to_string_lossy().into_owned()),
        claude: read_claude(&claude_path()),
        codex: read_codex(&codex_path()),
        claude_config: claude_path().to_string_lossy().into_owned(),
        codex_config: codex_path().to_string_lossy().into_owned(),
    }
}

/// Registra `harpoon mcp` para la colección en el asistente indicado (`claude` o `codex`).
#[tauri::command]
pub fn register_mcp(
    root: PathBuf,
    assistant: String,
    allow_write: bool,
) -> Result<McpStatus, String> {
    let cli = find_cli().ok_or("No se encuentra la CLI `harpoon`. Instálala con `cargo install --path crates/harpoon-cli`.")?;
    let command = cli.to_string_lossy().into_owned();
    let args = args_for(&root, allow_write);
    match assistant.as_str() {
        "claude" => write_claude(&claude_path(), Some((&command, &args)))?,
        "codex" => write_codex(&codex_path(), Some((&command, &args)))?,
        other => return Err(format!("asistente desconocido: {other}")),
    }
    Ok(mcp_status())
}

#[tauri::command]
pub fn unregister_mcp(assistant: String) -> Result<McpStatus, String> {
    match assistant.as_str() {
        "claude" => write_claude(&claude_path(), None)?,
        "codex" => write_codex(&codex_path(), None)?,
        other => return Err(format!("asistente desconocido: {other}")),
    }
    Ok(mcp_status())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claude_conserva_el_resto_y_el_orden() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".claude.json");
        std::fs::write(
            &path,
            r#"{"zeta": 1, "mcpServers": {"otro": {"command": "x"}}, "alfa": [1, 2]}"#,
        )
        .unwrap();
        let args = args_for(Path::new("/c/api"), true);
        write_claude(&path, Some(("/bin/harpoon", &args))).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(
            text.find("zeta").unwrap() < text.find("alfa").unwrap(),
            "orden: {text}"
        );
        assert!(text.contains("\"otro\""), "{text}");
        assert_eq!(
            read_claude(&path),
            Some(Registration {
                collection: Some("/c/api".into()),
                allow_write: true,
                command: "/bin/harpoon".into()
            })
        );
        write_claude(&path, None).unwrap();
        assert_eq!(read_claude(&path), None);
        assert!(std::fs::read_to_string(&path).unwrap().contains("\"otro\""));
    }

    #[test]
    fn codex_conserva_comentarios_y_otros_servidores() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(
            &path,
            "# mi config\nmodel = \"gpt\"\n\n[mcp_servers.godot]\ncommand = \"godot\"\n",
        )
        .unwrap();
        let args = args_for(Path::new("/c/api"), false);
        write_codex(&path, Some(("/bin/harpoon", &args))).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.starts_with("# mi config\nmodel = \"gpt\""), "{text}");
        assert!(
            text.contains("[mcp_servers.godot]") && text.contains("[mcp_servers.harpoon]"),
            "{text}"
        );
        assert_eq!(
            read_codex(&path).unwrap().collection.as_deref(),
            Some("/c/api")
        );
        write_codex(&path, None).unwrap();
        assert_eq!(read_codex(&path), None);
        // Sin archivo previo también funciona.
        let fresh = dir.path().join("nuevo").join("config.toml");
        write_codex(&fresh, Some(("/bin/harpoon", &args))).unwrap();
        assert!(read_codex(&fresh).is_some());
    }
}
