//! Importación local de documentos `OpenAPI` 3 en JSON o YAML.
use crate::{Error, Result, postman};
use serde_json::{Value, json};
use std::{fs, path::Path};
fn err(message: impl Into<String>) -> Error {
    Error::Exchange(message.into())
}
fn val<'a>(doc: &'a Value, object: &'a Value) -> Option<&'a Value> {
    match object.get("$ref").and_then(Value::as_str) {
        Some(reference) => reference.strip_prefix('#').and_then(|p| doc.pointer(p)),
        None => Some(object),
    }
}
fn strv(v: &Value) -> String {
    v.as_str().map_or_else(
        || {
            if v.is_null() {
                String::new()
            } else {
                v.to_string()
            }
        },
        str::to_owned,
    )
}
fn server(v: &Value, warnings: &mut Vec<String>) -> String {
    let mut url = v["url"].as_str().unwrap_or("").to_owned();
    if let Some(vars) = v["variables"].as_object() {
        for (key, value) in vars {
            url = url.replace(&format!("{{{key}}}"), &strv(&value["default"]));
        }
    }
    if url.is_empty() || url.starts_with('/') {
        warnings.push("Servidor relativo o ausente: configura {{base_url}} antes de enviar".into());
        url = format!("{{{{base_url}}}}{url}");
    }
    url.trim_end_matches('/').to_owned()
}
fn sample(param: &Value) -> String {
    if !param["example"].is_null() {
        strv(&param["example"])
    } else if !param["schema"]["example"].is_null() {
        strv(&param["schema"]["example"])
    } else if !param["schema"]["default"].is_null() {
        strv(&param["schema"]["default"])
    } else {
        format!("{{{{{}}}}}", strv(&param["name"]))
    }
}
#[allow(
    clippy::too_many_lines,
    clippy::missing_panics_doc,
    clippy::if_not_else
)]
pub fn read(path: &Path) -> Result<postman::Preview> {
    let meta = fs::metadata(path).map_err(|e| err(e.to_string()))?;
    if meta.len() > 20 * 1024 * 1024 {
        return Err(err("archivo OpenAPI demasiado grande (máximo 20 MB)"));
    }
    let data = fs::read(path).map_err(|e| err(e.to_string()))?;
    let doc: Value =
        serde_yaml::from_slice(&data).map_err(|e| err(format!("OpenAPI no válido: {e}")))?;
    let version = doc["openapi"].as_str().unwrap_or("");
    if !version.starts_with("3.0.") && !version.starts_with("3.1.") {
        return Err(err("se requiere OpenAPI 3.0 o 3.1"));
    }
    let name = doc["info"]["title"]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| err("OpenAPI no tiene título"))?;
    let paths = doc["paths"]
        .as_object()
        .ok_or_else(|| err("OpenAPI no tiene rutas"))?;
    let mut warnings = Vec::new();
    let root_server = doc["servers"]
        .as_array()
        .and_then(|s| s.first())
        .cloned()
        .unwrap_or(Value::Null);
    if doc["servers"].as_array().is_some_and(|s| s.len() > 1) {
        warnings.push("Solo se usa el primer servidor de cada nivel".into());
    }
    if doc["security"].as_array().is_some_and(|s| !s.is_empty()) {
        warnings.push("La autenticación OpenAPI requiere configuración manual".into());
    }
    let mut items = Vec::new();
    for (path, path_item) in paths {
        let Some(path_item) = val(&doc, path_item) else {
            warnings.push(format!("{path}: referencia externa o no resuelta"));
            continue;
        };
        for (method, operation) in path_item.as_object().into_iter().flatten() {
            if !matches!(
                method.as_str(),
                "get" | "post" | "put" | "patch" | "delete" | "head" | "options"
            ) {
                continue;
            }
            if !operation.is_object() {
                continue;
            }
            let title = operation["summary"]
                .as_str()
                .or_else(|| operation["operationId"].as_str())
                .map_or_else(
                    || format!("{} {path}", method.to_uppercase()),
                    str::to_owned,
                );
            if operation["security"]
                .as_array()
                .is_some_and(|s| !s.is_empty())
            {
                warnings.push(format!("{title}: configura la autenticación manualmente"));
            }
            let selected = operation["servers"]
                .as_array()
                .and_then(|s| s.first())
                .or_else(|| path_item["servers"].as_array().and_then(|s| s.first()))
                .unwrap_or(&root_server);
            let mut url = format!("{}{path}", server(selected, &mut warnings));
            let mut query = Vec::new();
            let mut headers = Vec::new();
            let mut params: Vec<Value> = Vec::new();
            for source in [&path_item["parameters"], &operation["parameters"]] {
                for p in source.as_array().into_iter().flatten() {
                    if let Some(p) = val(&doc, p) {
                        if let Some(i) = params
                            .iter()
                            .position(|x| x["name"] == p["name"] && x["in"] == p["in"])
                        {
                            params.remove(i);
                        }
                        params.push(p.clone());
                    } else {
                        warnings.push(format!("{title}: referencia de parámetro no resuelta"));
                    }
                }
            }
            for p in params {
                let key = strv(&p["name"]);
                let value = sample(&p);
                match p["in"].as_str().unwrap_or("") {
                    "path" => {
                        url = url.replace(&format!("{{{key}}}"), &format!("{{{{{key}}}}}"));
                        warnings.push(format!(
                            "{title}: configura la variable de ruta {{{{{key}}}}}"
                        ));
                    }
                    "query" => query.push(json!({"key":key,"value":value})),
                    "header" => headers.push(json!({"key":key,"value":value})),
                    "cookie" => warnings.push(format!(
                        "{title}: parámetro cookie {key} requiere configuración manual"
                    )),
                    _ => warnings.push(format!("{title}: parámetro {key} no compatible")),
                }
            }
            let mut request = json!({"method":method.to_uppercase(),"url":{"raw":url,"query":query},"header":headers,"description":operation["description"]});
            if !operation["requestBody"].is_null() {
                if let Some(body) = val(&doc, &operation["requestBody"]) {
                    if let Some(content) = body["content"].as_object() {
                        let choice = content
                            .get_key_value("application/json")
                            .or_else(|| content.iter().next());
                        if let Some((kind, media)) = choice {
                            if !kind.contains("json")
                                && !kind.contains("xml")
                                && !kind.starts_with("text/")
                            {
                                warnings.push(format!(
                                    "{title}: tipo de cuerpo {kind} requiere configuración manual"
                                ));
                            } else {
                                let example = if !media["example"].is_null() {
                                    &media["example"]
                                } else {
                                    &media["schema"]["example"]
                                };
                                if !example.is_null() {
                                    let value = if example.is_string() {
                                        strv(example)
                                    } else {
                                        serde_json::to_string_pretty(example)?
                                    };
                                    request["body"] = json!({"mode":"raw","raw":value,"options":{"raw":{"language":if kind.contains("json") {"json"} else if kind.contains("xml") {"xml"} else {"text"}}}});
                                    request["header"]
                                        .as_array_mut()
                                        .expect("headers")
                                        .push(json!({"key":"Content-Type","value":kind}));
                                } else {
                                    warnings.push(format!(
                                        "{title}: cuerpo sin ejemplo; complétalo manualmente"
                                    ));
                                }
                            }
                        }
                    }
                } else {
                    warnings.push(format!("{title}: referencia de cuerpo no resuelta"));
                }
            }
            let item = json!({"name":title,"request":request});
            let tag = operation["tags"]
                .as_array()
                .and_then(|t| t.first())
                .and_then(Value::as_str);
            if let Some(tag) = tag {
                let index = items
                    .iter()
                    .position(|v: &Value| v["name"] == tag && v["item"].is_array())
                    .unwrap_or_else(|| {
                        items.push(json!({"name":tag,"item":[]}));
                        items.len() - 1
                    });
                items[index]["item"]
                    .as_array_mut()
                    .expect("items")
                    .push(item);
            } else {
                items.push(item);
            }
        }
    }
    if items.is_empty() {
        return Err(err("OpenAPI no contiene operaciones HTTP compatibles"));
    }
    let postman = json!({"info":{"name":name,"schema":"https://schema.getpostman.com/json/collection/v2.1.0/collection.json"},"item":items});
    let mut p = postman::parse(&postman)?;
    warnings.push(
        "Se importan solicitudes; esquemas, respuestas y validaciones OpenAPI no se convierten"
            .into(),
    );
    warnings.extend(p.warnings);
    p.warnings = warnings;
    Ok(p)
}
