//! Archivos de datos para `harpoon run --data`: CSV (con cabecera) o JSON
//! (lista de objetos). Cada fila se convierte en variables de la ejecución.

use std::path::Path;

use indexmap::IndexMap;

pub fn load(path: &Path) -> anyhow::Result<Vec<IndexMap<String, String>>> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "csv" => load_csv(path),
        "json" => load_json(path),
        other => anyhow::bail!("formato de datos no soportado: .{other} (usa .csv o .json)"),
    }
}

fn load_csv(path: &Path) -> anyhow::Result<Vec<IndexMap<String, String>>> {
    let mut reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .flexible(true)
        .from_path(path)?;
    let headers: Vec<String> = reader.headers()?.iter().map(str::to_owned).collect();
    let mut rows = Vec::new();
    for record in reader.records() {
        let record = record?;
        let mut row = IndexMap::new();
        for (i, h) in headers.iter().enumerate() {
            row.insert(h.clone(), record.get(i).unwrap_or("").to_owned());
        }
        rows.push(row);
    }
    Ok(rows)
}

fn load_json(path: &Path) -> anyhow::Result<Vec<IndexMap<String, String>>> {
    let text = std::fs::read_to_string(path)?;
    let value: serde_json::Value = serde_json::from_str(&text)?;
    let items = value
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("el JSON debe ser una lista de objetos"))?;
    let mut rows = Vec::new();
    for item in items {
        let obj = item
            .as_object()
            .ok_or_else(|| anyhow::anyhow!("cada elemento debe ser un objeto"))?;
        let mut row = IndexMap::new();
        for (k, v) in obj {
            let s = match v {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            row.insert(k.clone(), s);
        }
        rows.push(row);
    }
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_y_json() {
        let dir = tempfile::tempdir().unwrap();
        let csv_path = dir.path().join("d.csv");
        std::fs::write(&csv_path, "user,age\nada, 36\nlin,41\n").unwrap();
        let rows = load(&csv_path).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["user"], "ada");
        assert_eq!(rows[0]["age"], "36");

        let json_path = dir.path().join("d.json");
        std::fs::write(&json_path, r#"[{"user":"ada","n":1,"tags":["a"]}]"#).unwrap();
        let rows = load(&json_path).unwrap();
        assert_eq!(rows[0]["n"], "1");
        assert_eq!(rows[0]["tags"], r#"["a"]"#);

        assert!(load(&dir.path().join("x.yaml")).is_err());
    }
}
