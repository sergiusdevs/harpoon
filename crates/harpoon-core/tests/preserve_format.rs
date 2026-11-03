//! Guardar desde Harpoon no borra lo que se escribió a mano en los TOML.
use harpoon_core::{Collection, Method};

#[test]
fn guardar_una_solicitud_conserva_comentarios_y_orden() {
    let temp = tempfile::tempdir().unwrap();
    let col = Collection::init(temp.path(), "demo").unwrap();
    let path = temp.path().join("requests/users.toml");
    std::fs::write(
        &path,
        r#"# Lista los usuarios activos (ver ticket API-142)
name = 'Usuarios'
method = "GET"
url = "{{base_url}}/users"   # paginado por defecto

[headers]
# Obligatorio desde la v2
Accept = "application/json"
"#,
    )
    .unwrap();

    let mut col = Collection::load(&col.root).unwrap();
    let mut req = col.requests["users"].clone();
    req.method = Method::Post;
    req.headers.insert("X-Trace".into(), "1".into());
    col.save_request("users", req).unwrap();

    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.starts_with("# Lista los usuarios activos (ver ticket API-142)\nname = 'Usuarios'\nmethod = \"POST\"\n"), "{text}");
    assert!(
        text.contains("url = \"{{base_url}}/users\"   # paginado por defecto"),
        "{text}"
    );
    assert!(
        text.contains("# Obligatorio desde la v2\nAccept = \"application/json\"\nX-Trace = \"1\""),
        "{text}"
    );

    // Y sigue cargando igual.
    let col = Collection::load(temp.path()).unwrap();
    assert_eq!(col.requests["users"].method, Method::Post);
    assert_eq!(col.requests["users"].headers["X-Trace"], "1");
}

#[test]
fn guardar_metadatos_conserva_comentarios_de_harpoon_toml() {
    let temp = tempfile::tempdir().unwrap();
    let col = Collection::init(temp.path(), "demo").unwrap();
    let meta_path = temp.path().join("harpoon.toml");
    let original = std::fs::read_to_string(&meta_path).unwrap();
    std::fs::write(
        &meta_path,
        format!("# Colección del equipo de pagos\n{original}"),
    )
    .unwrap();

    let mut col = Collection::load(&col.root).unwrap();
    let mut meta = col.meta.clone();
    meta.name = "Pagos".into();
    col.save_meta(meta).unwrap();

    let text = std::fs::read_to_string(&meta_path).unwrap();
    assert!(
        text.starts_with("# Colección del equipo de pagos\n"),
        "{text}"
    );
    assert!(text.contains("name = \"Pagos\""), "{text}");
}
