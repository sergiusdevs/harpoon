//! Imprime el JSON que recibe el frontend para una colección recién creada.
//! Útil para depurar la UI sin abrir la app: `cargo run -p harpoon-desktop --example dump`.

use harpoon_core::Collection;

#[derive(serde::Serialize)]
struct Dto {
    root: std::path::PathBuf,
    meta: harpoon_core::model::CollectionMeta,
    environments: indexmap::IndexMap<String, harpoon_core::Environment>,
    requests: indexmap::IndexMap<String, harpoon_core::Request>,
}

fn main() {
    let dir = std::env::temp_dir().join("harpoon-dump");
    let _ = std::fs::remove_dir_all(&dir);
    let col = Collection::init(&dir, "api-local").expect("init");
    let dto = Dto {
        root: col.root,
        meta: col.meta,
        environments: col.environments,
        requests: col.requests,
    };
    println!("{}", serde_json::to_string_pretty(&dto).expect("json"));
}
