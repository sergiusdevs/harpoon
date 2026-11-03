use harpoon_core::{Body, Collection, Method, Request, RequestType};

#[test]
fn guarda_y_recarga_tipos_explicitos_con_urls_variables() {
    let tmp = tempfile::tempdir().unwrap();
    let mut collection = Collection::init(tmp.path(), "Protocolos").unwrap();
    for (name, kind) in [
        ("http", RequestType::Http),
        ("graphql", RequestType::Graphql),
        ("ws", RequestType::Websocket),
        ("sse", RequestType::Sse),
    ] {
        let mut request = Request::new(name, Method::Get, "{{endpoint}}");
        request.request_type = Some(kind);
        collection.save_request(name, request).unwrap();
    }
    let loaded = Collection::load(tmp.path()).unwrap();
    assert_eq!(loaded.requests["http"].request_type(), RequestType::Http);
    assert_eq!(
        loaded.requests["graphql"].request_type(),
        RequestType::Graphql
    );
    assert_eq!(loaded.requests["ws"].request_type(), RequestType::Websocket);
    assert_eq!(loaded.requests["sse"].request_type(), RequestType::Sse);
    let raw = std::fs::read_to_string(tmp.path().join("requests/ws.toml")).unwrap();
    assert!(raw.contains("type = \"websocket\""));
}

#[test]
fn archivos_antiguos_se_reconocen_sin_migracion() {
    let mut request: Request =
        toml::from_str("name = 'WS'\nurl = 'WSS://localhost/socket'").unwrap();
    assert_eq!(request.request_type(), RequestType::Websocket);
    request.url = "https://localhost/events".into();
    request.headers.insert(
        "accept".into(),
        "application/json, text/event-stream; charset=utf-8".into(),
    );
    assert_eq!(request.request_type(), RequestType::Sse);
    request.headers.clear();
    request.body = Body::Graphql {
        schema: String::new(),
        query: "{ user { id } }".into(),
        variables: "{}".into(),
        operation_name: String::new(),
    };
    assert_eq!(request.request_type(), RequestType::Graphql);
    request.request_type = Some(RequestType::Http);
    assert_eq!(request.request_type(), RequestType::Http);
}

#[test]
fn conserva_descripciones_y_esquema_local_al_guardar() {
    let tmp = tempfile::tempdir().unwrap();
    let mut col = Collection::init(tmp.path(), "Documentación").unwrap();
    let mut request = Request::new("Consulta", Method::Post, "http://localhost/graphql");
    request.query.insert("page".into(), "0".into());
    request
        .query_descriptions
        .insert("page".into(), "Página inicial".into());
    request
        .header_descriptions
        .insert("Accept".into(), "Formato esperado".into());
    request.body = Body::Graphql {
        query: "{ greeting }".into(),
        variables: "{}".into(),
        operation_name: String::new(),
        schema: "type Query { greeting: String }".into(),
    };
    col.save_request("query", request.clone()).unwrap();
    assert_eq!(
        Collection::load(tmp.path()).unwrap().requests["query"],
        request
    );
}

#[test]
fn descripciones_de_secretos_se_guardan_fuera_del_archivo_publico() {
    let tmp = tempfile::tempdir().unwrap();
    let mut col = Collection::init(tmp.path(), "Entornos").unwrap();
    let mut env = harpoon_core::Environment::default();
    env.vars
        .insert("base_url".into(), "http://localhost".into());
    env.vars.insert("token".into(), "test-secret-value".into());
    env.secrets.push("token".into());
    env.descriptions
        .insert("base_url".into(), "Servidor de desarrollo".into());
    env.descriptions
        .insert("token".into(), "Descripción local del token".into());
    col.save_environment("dev", env.clone()).unwrap();
    let public = std::fs::read_to_string(tmp.path().join("environments/dev.toml")).unwrap();
    assert!(!public.contains("test-secret-value"));
    assert!(!public.contains("Descripción local del token"));
    let secret = std::fs::read_to_string(tmp.path().join("environments/dev.secrets.toml")).unwrap();
    assert!(secret.contains("test-secret-value"));
    assert!(secret.contains("Descripción local del token"));
    assert_eq!(
        Collection::load(tmp.path()).unwrap().environments["dev"],
        env
    );
}

#[test]
fn postman_conserva_cabeceras_desactivadas_y_documentadas() {
    let tmp = tempfile::tempdir().unwrap();
    let mut col = Collection::init(&tmp.path().join("source"), "Cabeceras").unwrap();
    let mut request = Request::new("Headers", Method::Get, "http://localhost/echo");
    request
        .headers
        .insert("X-Debug".into(), "{{missing}}".into());
    request.disabled_headers.push("x-debug".into());
    request
        .header_descriptions
        .insert("X-Debug".into(), "Desactivada".into());
    col.save_request("headers", request).unwrap();
    let (document, _) = harpoon_core::postman::export(&col);
    let path = tmp.path().join("collection.json");
    std::fs::write(&path, serde_json::to_vec(&document).unwrap()).unwrap();
    let preview = harpoon_core::postman::read(&path).unwrap();
    let imported = harpoon_core::postman::import(&preview, &tmp.path().join("target")).unwrap();
    let request = imported
        .requests
        .values()
        .find(|request| request.name == "Headers")
        .unwrap();
    assert!(request.header_is_disabled("X-DEBUG"));
    assert_eq!(request.headers["X-Debug"], "{{missing}}");
    assert_eq!(request.header_descriptions["X-Debug"], "Desactivada");
}
