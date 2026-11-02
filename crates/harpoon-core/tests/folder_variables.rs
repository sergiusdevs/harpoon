//! Comportamiento público de las variables de carpeta al guardar y cargar colecciones.
use harpoon_core::{Collection, Environment, Method, Request};
use indexmap::IndexMap;

fn vars(pairs: &[(&str, &str)]) -> IndexMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).into(), (*v).into()))
        .collect()
}

#[test]
fn carpetas_anidadas_respetan_prioridad_y_se_cargan_de_disco() {
    let temp = tempfile::tempdir().unwrap();
    let mut col = Collection::init(temp.path(), "test").unwrap();
    col.meta.default_environment = None;
    col.meta.vars = vars(&[("origin", "global"), ("global_only", "g")]);
    col.save_meta(col.meta.clone()).unwrap();
    col.create_folder("api/users").unwrap();
    col.save_folder_vars("api", vars(&[("origin", "parent"), ("parent_only", "p")]))
        .unwrap();
    col.save_folder_vars(
        "api/users",
        vars(&[("origin", "child"), ("child_only", "c")]),
    )
    .unwrap();
    col.save_request("api/users/list", Request::new("list", Method::Get, "/"))
        .unwrap();
    col.save_environment(
        "prod",
        Environment {
            vars: vars(&[("origin", "env")]),
            secrets: vec![],
        },
    )
    .unwrap();

    let col = Collection::load(temp.path()).unwrap();
    assert!(!col.requests.contains_key("api/folder"));
    let folder_scope = col.scope_for_request(None, "api/users/list").unwrap();
    assert_eq!(
        folder_scope
            .interpolate("{{origin}}/{{global_only}}/{{parent_only}}/{{child_only}}")
            .unwrap(),
        "child/g/p/c"
    );
    assert_eq!(
        col.scope_for_request(None, "api/other")
            .unwrap()
            .get("origin"),
        Some("parent")
    );
    assert_eq!(
        col.scope_for_request(None, "other").unwrap().get("origin"),
        Some("global")
    );
    let scope = col
        .scope_for_request(Some("prod"), "api/users/list")
        .unwrap();
    assert_eq!(scope.get("origin"), Some("env"));
    assert_eq!(
        scope
            .with_layer(vars(&[("origin", "runtime")]))
            .get("origin"),
        Some("runtime")
    );
}

#[test]
fn configuracion_sigue_la_carpeta_al_mover_y_borrar() {
    let temp = tempfile::tempdir().unwrap();
    let mut col = Collection::init(temp.path(), "test").unwrap();
    col.create_folder("api/users").unwrap();
    col.save_folder_vars("api/users", vars(&[("path", "/users")]))
        .unwrap();
    col.rename_folder("api", "v2").unwrap();
    assert_eq!(
        col.scope_for_request(None, "v2/users/list")
            .unwrap()
            .get("path"),
        Some("/users")
    );
    let loaded = Collection::load(temp.path()).unwrap();
    assert_eq!(loaded.folder_configs["v2/users"].vars["path"], "/users");
    col.delete_folder("v2").unwrap();
    assert!(col.folder_configs.is_empty());
}

#[test]
fn no_sobrescribe_solicitud_antigua_llamada_folder() {
    let temp = tempfile::tempdir().unwrap();
    Collection::init(temp.path(), "test").unwrap();
    std::fs::create_dir_all(temp.path().join("requests/api")).unwrap();
    let path = temp.path().join("requests/api/folder.toml");
    let request = "name = \"legado\"\nmethod = \"GET\"\nurl = \"/legacy\"\n";
    std::fs::write(&path, request).unwrap();
    let mut col = Collection::load(temp.path()).unwrap();
    assert!(col.requests.contains_key("api/folder"));
    assert!(
        col.save_folder_vars("api", vars(&[("path", "/api")]))
            .is_err()
    );
    assert_eq!(std::fs::read_to_string(path).unwrap(), request);
}
