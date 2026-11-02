//! Imprime el JSON que recibe el frontend para una colección recién creada.
use harpoon_core::Collection;
fn main() {
    let dir = std::env::temp_dir().join("harpoon-dump");
    let _ = std::fs::remove_dir_all(&dir);
    let col = Collection::init(&dir, "api-local").unwrap();
    #[derive(serde::Serialize)]
    struct Dto { root: std::path::PathBuf, meta: harpoon_core::model::CollectionMeta, environments: Vec<String>, requests: indexmap::IndexMap<String, harpoon_core::Request> }
    let dto = Dto { root: col.root, meta: col.meta, environments: col.environments.keys().cloned().collect(), requests: col.requests };
    println!("{}", serde_json::to_string_pretty(&dto).unwrap());
}
