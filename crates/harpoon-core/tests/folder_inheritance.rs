//! Herencia por carpeta: cabeceras, auth y scripts en `folder.toml`.
use harpoon_core::{Auth, Collection, Method, Request, Scripts};
use indexmap::IndexMap;

fn headers(pairs: &[(&str, &str)]) -> IndexMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).into(), (*v).into()))
        .collect()
}

fn collection() -> (tempfile::TempDir, Collection) {
    let temp = tempfile::tempdir().unwrap();
    let mut col = Collection::init(temp.path(), "test").unwrap();
    col.create_folder("api/admin").unwrap();
    col.save_folder_settings(
        "api",
        headers(&[("X-Team", "core"), ("Accept", "application/json")]),
        Auth::Bearer {
            token: "{{api_token}}".into(),
        },
        Scripts {
            pre: "harpoon.vars.set('api', '1');".into(),
            ..Scripts::default()
        },
    )
    .unwrap();
    col.save_folder_settings(
        "api/admin",
        headers(&[("x-team", "admin"), ("X-Admin", "1")]),
        Auth::Inherit,
        Scripts::default(),
    )
    .unwrap();
    col.save_request("api/admin/users", Request::new("users", Method::Get, "/u"))
        .unwrap();
    (temp, col)
}

#[test]
fn se_guarda_en_folder_toml_y_se_recarga() {
    let (temp, _) = collection();
    let text = std::fs::read_to_string(temp.path().join("requests/api/folder.toml")).unwrap();
    assert!(text.contains("[headers]") && text.contains("[auth]") && text.contains("[scripts]"));
    let admin =
        std::fs::read_to_string(temp.path().join("requests/api/admin/folder.toml")).unwrap();
    assert!(!admin.contains("[auth]"), "inherit no se escribe: {admin}");

    let col = Collection::load(temp.path()).unwrap();
    assert_eq!(col.folder_configs["api"].headers["X-Team"], "core");
    assert!(matches!(
        col.folder_configs["api"].auth,
        Auth::Bearer { .. }
    ));
    assert_ne!(col.folder_configs["api"].scripts.pre, "");
}

#[test]
fn cabeceras_de_fuera_a_dentro_y_la_solicitud_manda() {
    let (_temp, col) = collection();
    let mut req = col.requests["api/admin/users"].clone();
    req.headers.insert("accept".into(), "text/plain".into());

    let out = col.with_folder_inheritance("api/admin/users", &req);
    let names: Vec<_> = out.headers.keys().map(String::as_str).collect();
    // La interior sustituye a la exterior sin importar mayúsculas; las propias van al final.
    assert_eq!(names, ["x-team", "X-Admin", "accept"]);
    assert_eq!(out.headers["x-team"], "admin");
    assert_eq!(out.headers["accept"], "text/plain");
}

#[test]
fn desactivar_en_la_solicitud_quita_la_heredada() {
    let (_temp, col) = collection();
    let mut req = col.requests["api/admin/users"].clone();
    req.disabled_headers.push("X-ADMIN".into());
    let out = col.with_folder_inheritance("api/admin/users", &req);
    assert!(!out.headers.contains_key("X-Admin"));
}

#[test]
fn auth_de_la_carpeta_mas_cercana_que_la_defina() {
    let (_temp, col) = collection();
    let req = col.requests["api/admin/users"].clone();
    let out = col.with_folder_inheritance("api/admin/users", &req);
    assert_eq!(
        out.auth,
        Auth::Bearer {
            token: "{{api_token}}".into()
        }
    );

    // Una auth propia (incluida "none") no se toca.
    let mut own = req.clone();
    own.auth = Auth::None;
    assert_eq!(
        col.with_folder_inheritance("api/admin/users", &own).auth,
        Auth::None
    );

    // Fuera de las carpetas no hay nada que heredar.
    let root = Request::new("root", Method::Get, "/");
    assert_eq!(col.with_folder_inheritance("root", &root), root);
}

#[test]
fn el_detector_revisa_la_configuracion_de_carpeta() {
    let (_temp, mut col) = collection();
    col.save_folder_settings(
        "api/admin",
        headers(&[("Authorization", "Bearer abcdefghijklmnopqrstuvwxyz123456")]),
        Auth::Inherit,
        Scripts::default(),
    )
    .unwrap();
    let findings = harpoon_core::scan::scan_collection(&col);
    assert!(
        findings
            .iter()
            .any(|f| f.file == "requests/api/admin/folder.toml"),
        "{findings:?}"
    );
}

#[test]
fn color_de_carpeta_en_folder_toml() {
    let (temp, mut col) = collection();
    col.set_folder_color("api", Some("Teal".into())).unwrap();
    col.set_folder_color("api/admin", Some("#ff8800".into()))
        .unwrap();
    assert!(col.set_folder_color("api", Some("fucsia".into())).is_err());
    assert!(col.set_folder_color("api", Some("#12345".into())).is_err());

    let text = std::fs::read_to_string(temp.path().join("requests/api/folder.toml")).unwrap();
    assert!(text.contains("color = \"teal\""), "{text}");
    assert!(text.contains("[headers]"), "conserva el resto: {text}");
    let col = Collection::load(temp.path()).unwrap();
    assert_eq!(col.folder_configs["api"].color.as_deref(), Some("teal"));
    assert_eq!(
        col.folder_configs["api/admin"].color.as_deref(),
        Some("#ff8800")
    );

    let mut col = col;
    col.set_folder_color("api", None).unwrap();
    let text = std::fs::read_to_string(temp.path().join("requests/api/folder.toml")).unwrap();
    assert!(!text.contains("color"), "{text}");
}
