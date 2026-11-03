//! `validate` encuentra los problemas típicos sin enviar nada.
use harpoon_core::validate::{Severity, validate, variable_references};
use harpoon_core::{Body, Collection, Method, Request};

fn messages(col: &Collection) -> Vec<String> {
    validate(col, None)
        .into_iter()
        .map(|i| format!("{:?} {} {}", i.severity, i.location, i.message))
        .collect()
}

#[test]
fn una_coleccion_nueva_es_valida() {
    let temp = tempfile::tempdir().unwrap();
    let col = Collection::init(temp.path(), "demo").unwrap();
    assert_eq!(messages(&col), Vec::<String>::new());
}

#[test]
fn detecta_variables_url_json_archivos_y_status() {
    let temp = tempfile::tempdir().unwrap();
    let mut col = Collection::init(temp.path(), "demo").unwrap();

    let mut req = Request::new("roto", Method::Post, "{{base_url}}/x/{{falta}}");
    req.headers
        .insert("X-Id".into(), "{{otra}} {{$uuid}}".into());
    req.body = Body::Json {
        json: "{ no es json".into(),
    };
    req.expect.status = Some(42);
    col.save_request("roto", req).unwrap();

    col.save_request("ftp", Request::new("ftp", Method::Get, "ftp://archivos"))
        .unwrap();
    let mut file = Request::new("subir", Method::Post, "{{base_url}}/subir");
    file.body = Body::Binary {
        path: "no-existe.bin".into(),
    };
    col.save_request("subir", file).unwrap();

    // Una variable creada por un script no cuenta como "sin definir".
    let mut login = Request::new("login", Method::Post, "{{base_url}}/login");
    login.scripts.post = "harpoon.vars.set('token', harpoon.res.json().token);".into();
    col.save_request("login", login).unwrap();
    let mut me = Request::new("me", Method::Get, "{{base_url}}/me");
    me.headers
        .insert("Authorization".into(), "Bearer {{token}}".into());
    col.save_request("me", me).unwrap();

    let found = messages(&col);
    let has = |needle: &str| found.iter().any(|m| m.contains(needle));
    assert!(
        has("Error url la variable `{{falta}}` no está definida"),
        "{found:#?}"
    );
    assert!(
        has("Error header X-Id la variable `{{otra}}`"),
        "{found:#?}"
    );
    assert!(!has("$uuid") && !has("`{{token}}`"), "{found:#?}");
    assert!(has("Warning body JSON no válido"), "{found:#?}");
    assert!(
        has("Error expect.status 42 no es un código HTTP válido"),
        "{found:#?}"
    );
    assert!(has("Error url esquema `ftp` no válido"), "{found:#?}");
    assert!(has("no existe el archivo `no-existe.bin`"), "{found:#?}");

    let errors = validate(&col, None)
        .iter()
        .filter(|i| i.severity == Severity::Error)
        .count();
    assert_eq!(errors, 5, "{found:#?}");
}

#[test]
fn entorno_inexistente_es_un_error() {
    let temp = tempfile::tempdir().unwrap();
    let col = Collection::init(temp.path(), "demo").unwrap();
    let issues = validate(&col, Some("prod"));
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].message, "el entorno `prod` no existe");
}

#[test]
fn referencias_ignoran_dinamicas_y_llaves_sueltas() {
    assert_eq!(
        variable_references("{{a}}/{{ b }}/{{$uuid}}/{{sin cierre"),
        ["a", "b"]
    );
}
