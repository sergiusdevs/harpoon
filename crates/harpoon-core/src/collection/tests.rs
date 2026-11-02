use super::*;

#[test]
fn init_y_load_son_simetricos() {
    let tmp = tempfile::tempdir().unwrap();
    let col = Collection::init(tmp.path(), "demo").unwrap();
    assert_eq!(col.meta.name, "demo");
    assert_eq!(col.requests.len(), 1);
    assert!(col.requests.contains_key("health"));
    assert!(col.environments.contains_key("dev"));

    let scope = col.scope(None).unwrap();
    assert_eq!(
        scope.interpolate(&col.requests["health"].url).unwrap(),
        "http://localhost:8080/health"
    );
}

#[test]
fn discover_sube_directorios_y_fusiona_secretos() {
    let tmp = tempfile::tempdir().unwrap();
    Collection::init(tmp.path(), "demo").unwrap();
    fs::write(
        tmp.path().join("environments/dev.secrets.toml"),
        "[vars]\ntoken = \"s3cr3t\"\n",
    )
    .unwrap();
    let nested = tmp.path().join("requests/users");
    fs::create_dir_all(&nested).unwrap();

    let col = Collection::discover(&nested).unwrap();
    assert_eq!(col.root, tmp.path());
    assert_eq!(col.environments["dev"].vars["token"], "s3cr3t");
}

#[test]
fn crear_renombrar_y_borrar_solicitudes_y_carpetas() {
    let tmp = tempfile::tempdir().unwrap();
    let mut col = Collection::init(tmp.path(), "demo").unwrap();

    let req = Request::new("Listar", crate::Method::Get, "{{base_url}}/users");
    col.save_request("users/list", req.clone()).unwrap();
    assert!(tmp.path().join("requests/users/list.toml").is_file());

    col.rename_request("users/list", "users/all").unwrap();
    assert!(!tmp.path().join("requests/users/list.toml").exists());
    assert!(col.requests.contains_key("users/all"));

    col.rename_folder("users", "people").unwrap();
    assert!(tmp.path().join("requests/people/all.toml").is_file());
    assert!(!tmp.path().join("requests/users").exists());
    assert!(col.requests.contains_key("people/all"));

    col.delete_request("people/all").unwrap();
    assert!(
        !tmp.path().join("requests/people").exists(),
        "carpeta vacía podada"
    );
    assert!(!col.requests.contains_key("people/all"));

    col.create_folder("admin/audit").unwrap();
    col.save_request("admin/audit/log", req).unwrap();
    col.delete_folder("admin").unwrap();
    assert!(!tmp.path().join("requests/admin").exists());
    assert!(col.requests.is_empty() || !col.requests.keys().any(|k| k.starts_with("admin/")));

    // Recargar desde disco coincide con memoria.
    let reloaded = Collection::load(tmp.path()).unwrap();
    assert_eq!(
        reloaded.requests.keys().collect::<Vec<_>>(),
        col.requests.keys().collect::<Vec<_>>()
    );
}

#[test]
fn nombres_peligrosos_se_rechazan() {
    let tmp = tempfile::tempdir().unwrap();
    let mut col = Collection::init(tmp.path(), "demo").unwrap();
    assert!(matches!(
        col.delete_request("../x"),
        Err(Error::InvalidName(_))
    ));
    assert!(matches!(
        col.create_folder("etc/../../x"),
        Err(Error::InvalidName(_))
    ));
    assert!(matches!(
        col.rename_request("health", "a//b"),
        Err(Error::InvalidName(_))
    ));
    assert!(matches!(
        col.save_environment("dev/x", Environment::default()),
        Err(Error::InvalidName(_))
    ));
}

#[test]
fn guardar_entorno_separa_secretos() {
    let tmp = tempfile::tempdir().unwrap();
    let mut col = Collection::init(tmp.path(), "demo").unwrap();
    let mut env = Environment::default();
    env.vars.insert("base_url".into(), "https://api".into());
    env.vars.insert("token".into(), "s3cr3t".into());
    env.secrets.push("token".into());
    col.save_environment("prod", env).unwrap();

    let public = fs::read_to_string(tmp.path().join("environments/prod.toml")).unwrap();
    assert!(public.contains("base_url"));
    assert!(
        !public.contains("s3cr3t"),
        "el secreto no va al archivo versionado"
    );
    let secret = fs::read_to_string(tmp.path().join("environments/prod.secrets.toml")).unwrap();
    assert!(secret.contains("s3cr3t"));

    let reloaded = Collection::load(tmp.path()).unwrap();
    assert_eq!(reloaded.environments["prod"].vars["token"], "s3cr3t");

    col.delete_environment("prod").unwrap();
    assert!(!tmp.path().join("environments/prod.secrets.toml").exists());
    assert!(!col.environments.contains_key("prod"));
}

#[test]
fn borrar_entorno_predeterminado_limpia_meta() {
    let tmp = tempfile::tempdir().unwrap();
    let mut col = Collection::init(tmp.path(), "demo").unwrap();
    col.delete_environment("dev").unwrap();
    assert_eq!(col.meta.default_environment, None);
    let reloaded = Collection::load(tmp.path()).unwrap();
    assert_eq!(reloaded.meta.default_environment, None);
}
