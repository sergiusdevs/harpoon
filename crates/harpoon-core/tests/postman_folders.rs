//! Auth y scripts de carpetas Postman ↔ `folder.toml`.
use harpoon_core::{Auth, postman};
use serde_json::json;

#[test]
fn importa_y_exporta_auth_y_scripts_de_carpeta() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("api.postman_collection.json");
    let collection = json!({
        "info": {"name": "API", "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"},
        "item": [{
            "name": "admin",
            "auth": {"type": "bearer", "bearer": [{"key": "token", "value": "{{admin_token}}"}]},
            "event": [{"listen": "prerequest", "script": {"exec": ["console.log('admin');"]}}],
            "item": [
                {"name": "users", "item": [
                    {"name": "list", "request": {"method": "GET", "url": {"raw": "{{base_url}}/admin/users"}}}
                ]},
                {"name": "public", "request": {"method": "GET", "auth": {"type": "noauth"}, "url": {"raw": "{{base_url}}/ping"}}}
            ]
        }]
    });
    std::fs::write(&source, serde_json::to_vec(&collection).unwrap()).unwrap();

    let preview = postman::read(&source).unwrap();
    assert!(
        !preview.warnings.iter().any(|w| w.contains("no importados")),
        "{:?}",
        preview.warnings
    );
    let col = postman::import(&preview, &temp.path().join("api")).unwrap();

    let admin = &col.folder_configs["admin"];
    assert_eq!(
        admin.auth,
        Auth::Bearer {
            token: "{{admin_token}}".into()
        }
    );
    assert_eq!(admin.scripts.pre.trim(), "console.log('admin');");
    // La subcarpeta sin auth no crea folder.toml y la solicitud hereda.
    assert!(!col.folder_configs.contains_key("admin/users"));
    let list = &col.requests["admin/users/list"];
    assert_eq!(list.auth, Auth::Inherit);
    assert_eq!(
        col.with_folder_inheritance("admin/users/list", list).auth,
        admin.auth
    );
    assert_eq!(col.requests["admin/public"].auth, Auth::None);

    let (exported, _) = postman::export(&col);
    let folder = exported["item"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["name"] == "admin")
        .unwrap();
    assert_eq!(folder["auth"]["type"], "bearer");
    assert_eq!(folder["event"][0]["listen"], "prerequest");
}
