//! Secretos de colección y carpeta: el valor nunca llega a archivos versionados.

use harpoon_core::Collection;

fn read(root: &std::path::Path, rel: &str) -> String {
    std::fs::read_to_string(root.join(rel)).unwrap_or_default()
}

#[test]
fn secretos_de_coleccion_y_carpeta_quedan_fuera_de_git() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let mut c = Collection::init(root, "demo").unwrap();

    let mut meta = c.meta.clone();
    meta.vars
        .insert("base_url".into(), "https://api.x.com".into());
    meta.vars
        .insert("api_key".into(), "super-secreto-123".into());
    meta.secrets = vec!["api_key".into()];
    c.save_meta(meta).unwrap();

    c.create_folder("admin").unwrap();
    let vars = indexmap::IndexMap::from([
        ("admin_pass".to_string(), "pa55word!".to_string()),
        ("realm".to_string(), "x".to_string()),
    ]);
    c.save_folder_config("admin", vars, vec!["admin_pass".into()])
        .unwrap();

    let meta_file = read(root, "harpoon.toml");
    assert!(
        meta_file.contains("base_url")
            && meta_file.contains("api_key")
            && !meta_file.contains("super-secreto-123"),
        "{meta_file}"
    );
    let folder_file = read(root, "requests/admin/folder.toml");
    assert!(
        folder_file.contains("realm") && !folder_file.contains("pa55word"),
        "{folder_file}"
    );
    let local = read(root, ".harpoon/secrets.toml");
    assert!(
        local.contains("super-secreto-123") && local.contains("pa55word!"),
        "{local}"
    );
    assert!(read(root, ".harpoon/.gitignore").lines().any(|l| l == "*"));
    assert!(
        read(root, "environments/.gitignore")
            .lines()
            .any(|l| l == "*.secrets.toml")
    );

    // Al recargar, los valores vuelven a estar disponibles para enviar.
    let c = Collection::load(root).unwrap();
    assert_eq!(c.meta.vars["api_key"], "super-secreto-123");
    assert_eq!(c.folder_configs["admin"].vars["admin_pass"], "pa55word!");
    assert_eq!(c.missing_secrets(None), vec![]);

    // Renombrar la carpeta mueve sus secretos.
    let mut c = c;
    c.rename_folder("admin", "backoffice").unwrap();
    let c = Collection::load(root).unwrap();
    assert_eq!(
        c.folder_configs["backoffice"].vars["admin_pass"],
        "pa55word!"
    );
    assert!(!read(root, ".harpoon/secrets.toml").contains("[folders.admin]"));
}

#[test]
fn clon_sin_secretos_avisa_y_acepta_variables_del_sistema() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let mut c = Collection::init(root, "demo").unwrap();
    let mut meta = c.meta.clone();
    meta.vars
        .insert("harpoon_test_clone_tok".into(), "valor-local".into());
    meta.secrets = vec!["harpoon_test_clone_tok".into()];
    c.save_meta(meta).unwrap();

    // Simula un `git clone`: el archivo local no viaja.
    std::fs::remove_dir_all(root.join(".harpoon")).unwrap();
    let c = Collection::load(root).unwrap();
    let missing = c.missing_secrets(None);
    assert_eq!(missing.len(), 1);
    assert_eq!(missing[0].env_var, "HARPOON_HARPOON_TEST_CLONE_TOK");

    // SAFETY: test de un solo hilo para esta variable con nombre único.
    unsafe { std::env::set_var("HARPOON_HARPOON_TEST_CLONE_TOK", "desde-ci") };
    let c = Collection::load(root).unwrap();
    assert_eq!(c.missing_secrets(None), vec![]);
    let scope = c.scope(None).unwrap();
    assert_eq!(
        scope.interpolate("{{harpoon_test_clone_tok}}").unwrap(),
        "desde-ci"
    );
    unsafe { std::env::remove_var("HARPOON_HARPOON_TEST_CLONE_TOK") };
}

#[test]
fn postman_no_exporta_valores_secretos_e_importa_el_tipo_secret() {
    let dir = tempfile::tempdir().unwrap();
    let mut c = Collection::init(&dir.path().join("a"), "demo").unwrap();
    let mut meta = c.meta.clone();
    meta.vars
        .insert("base_url".into(), "https://api.x.com".into());
    meta.vars.insert("token".into(), "tok-super-secreto".into());
    meta.secrets = vec!["token".into()];
    c.save_meta(meta).unwrap();
    let c = Collection::load(&dir.path().join("a")).unwrap();

    let (json, warnings) = harpoon_core::postman::export(&c);
    let text = json.to_string();
    assert!(!text.contains("tok-super-secreto"), "{text}");
    assert!(text.contains(r#""type":"secret""#));
    assert!(warnings.iter().any(|w| w.contains("secretas")));

    let file = dir.path().join("p.json");
    std::fs::write(&file, text).unwrap();
    let preview = harpoon_core::postman::read(&file).unwrap();
    let imported = harpoon_core::postman::import(&preview, &dir.path().join("b")).unwrap();
    assert_eq!(imported.meta.secrets, vec!["token".to_string()]);
    assert!(!read(&dir.path().join("b"), "harpoon.toml").contains("tok-super-secreto"));
}

#[test]
fn exportar_avisa_de_secretos_escritos_tal_cual() {
    let dir = tempfile::tempdir().unwrap();
    let mut c = Collection::init(dir.path(), "demo").unwrap();
    let mut req = c.requests["health"].clone();
    req.headers
        .insert("Authorization".into(), "Bearer abcdef1234567890".into());
    c.save_request("health", req).unwrap();
    let shared = harpoon_core::share::export(&c).unwrap();
    assert!(
        shared
            .warnings
            .iter()
            .any(|w| w.contains("Posibles secretos") && w.contains("header Authorization")),
        "{:?}",
        shared.warnings
    );
}
