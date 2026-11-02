//! Intercambio con Postman Collection v2.1.
use crate::model::{ApiKeyLocation, MultipartPart, Scripts};
use crate::{Auth, Body, Collection, Error, Method, Request, Result};
use indexmap::IndexMap;
use serde::Serialize;
use serde_json::{Value, json};
use std::{fs, path::Path};

const SCHEMA: &str = "https://schema.getpostman.com/json/collection/v2.1.0/collection.json";
const LIMIT: u64 = 20 * 1024 * 1024;

#[derive(Serialize)]
pub struct Preview {
    pub name: String,
    pub warnings: Vec<String>,
    #[serde(skip)]
    requests: Vec<(String, Request)>,
    #[serde(skip)]
    folders: Vec<String>,
    #[serde(skip)]
    vars: IndexMap<String, String>,
    #[serde(skip)]
    auth: Auth,
    #[serde(skip)]
    scripts: Scripts,
}
fn err(s: impl Into<String>) -> Error {
    Error::Exchange(s.into())
}
fn strval(v: &Value) -> String {
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
fn text(v: &Value, key: &str) -> String {
    strval(&v[key])
}
fn slug(s: &str) -> String {
    let s: String = s
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || matches!(c, '-' | '_' | ' ' | '.') {
                c
            } else {
                '-'
            }
        })
        .collect();
    let s = s.trim().trim_matches('-').trim_matches('.');
    if s.is_empty() || s == "folder" {
        "item".into()
    } else {
        s.chars().take(80).collect()
    }
}
fn fields(v: &Value) -> IndexMap<String, String> {
    v.as_array()
        .into_iter()
        .flatten()
        .filter(|x| x["disabled"] != true)
        .filter_map(|x| {
            x["key"]
                .as_str()
                .map(|k| (k.to_owned(), strval(&x["value"])))
        })
        .collect()
}
fn auth(v: &Value, warnings: &mut Vec<String>, context: &str) -> Auth {
    let kind = text(v, "type");
    let f = fields(&v[&kind]);
    let get = |k: &str| f.get(k).cloned().unwrap_or_default();
    match kind.as_str() {
        "" => Auth::Inherit,
        "noauth" => Auth::None,
        "basic" => Auth::Basic {
            username: get("username"),
            password: get("password"),
        },
        "bearer" => Auth::Bearer {
            token: get("token"),
        },
        "digest" => Auth::Digest {
            username: get("username"),
            password: get("password"),
        },
        "apikey" => Auth::ApiKey {
            key: get("key"),
            value: get("value"),
            r#in: if get("in") == "query" {
                ApiKeyLocation::Query
            } else {
                ApiKeyLocation::Header
            },
        },
        _ => {
            warnings.push(format!(
                "{context}: autenticación {kind} no compatible; revísala antes de enviar"
            ));
            Auth::None
        }
    }
}
fn scripts(v: &Value, warnings: &mut Vec<String>, context: &str) -> Scripts {
    let mut out = Scripts::default();
    for e in v.as_array().into_iter().flatten() {
        let code = match &e["script"]["exec"] {
            Value::Array(a) => a.iter().map(strval).collect::<Vec<_>>().join("\n"),
            s => strval(s),
        };
        match text(e, "listen").as_str() {
            "prerequest" => {
                out.pre.push_str(&code);
                out.pre.push('\n');
            }
            "test" => {
                out.test.push_str(&code);
                out.test.push('\n');
            }
            _ => warnings.push(format!("{context}: evento de script no compatible")),
        }
    }
    if !out.is_empty() {
        warnings.push(format!(
            "{context}: comprueba la compatibilidad de los scripts Postman"
        ));
    }
    out
}
#[allow(clippy::too_many_lines, clippy::many_single_char_names)]
fn parse_items(
    items: &Value,
    parent: &str,
    inherited: &Auth,
    preview: &mut Preview,
    depth: usize,
) -> Result<()> {
    if depth > 32 {
        return Err(err("demasiadas carpetas anidadas"));
    }
    for item in items
        .as_array()
        .ok_or_else(|| err("lista de elementos Postman no válida"))?
    {
        let name = item["name"].as_str().unwrap_or("Solicitud");
        let base = slug(name);
        let mut key = if parent.is_empty() {
            base.clone()
        } else {
            format!("{parent}/{base}")
        };
        let original = key.clone();
        let mut n = 2;
        while preview.folders.contains(&key) || preview.requests.iter().any(|(k, _)| k == &key) {
            key = format!("{original}-{n}");
            n += 1;
        }
        if item["item"].is_array() {
            let a = if item["auth"].is_object() {
                auth(&item["auth"], &mut preview.warnings, name)
            } else {
                inherited.clone()
            };
            if item["event"].is_array() {
                preview
                    .warnings
                    .push(format!("{name}: scripts de carpeta no importados"));
            }
            preview.folders.push(key.clone());
            parse_items(&item["item"], &key, &a, preview, depth + 1)?;
            continue;
        }
        let r = &item["request"];
        let method = match text(r, "method").to_uppercase().as_str() {
            "GET" => Method::Get,
            "POST" => Method::Post,
            "PUT" => Method::Put,
            "PATCH" => Method::Patch,
            "DELETE" => Method::Delete,
            "HEAD" => Method::Head,
            "OPTIONS" => Method::Options,
            m => return Err(err(format!("{name}: método {m} no compatible"))),
        };
        let u = &r["url"];
        if u["variable"]
            .as_array()
            .is_some_and(|values| !values.is_empty())
        {
            preview.warnings.push(format!(
                "{name}: variables de ruta Postman no importadas; revisa la URL"
            ));
        }
        let raw = if u.is_string() {
            strval(u)
        } else {
            text(u, "raw")
        };
        if raw.is_empty() {
            return Err(err(format!("{name}: falta la URL raw")));
        }
        let mut req = Request::new(name, method, raw.clone());
        req.auth = if r["auth"].is_object() {
            auth(&r["auth"], &mut preview.warnings, name)
        } else {
            inherited.clone()
        };
        for h in r["header"].as_array().into_iter().flatten() {
            let k = text(h, "key");
            if h["disabled"] == true {
                preview
                    .warnings
                    .push(format!("{name}: cabecera desactivada {k} omitida"));
                continue;
            }
            if !k.is_empty() && req.headers.insert(k.clone(), text(h, "value")).is_some() {
                preview
                    .warnings
                    .push(format!("{name}: cabecera duplicada {k}"));
            }
        }
        if u["query"].as_array().is_some_and(|query| !query.is_empty()) {
            let (without_fragment, fragment) = raw
                .split_once('#')
                .map_or((raw.as_str(), None), |(u, f)| (u, Some(f)));
            without_fragment
                .split('?')
                .next()
                .unwrap_or(without_fragment)
                .clone_into(&mut req.url);
            if let Some(fragment) = fragment {
                req.url.push('#');
                req.url.push_str(fragment);
            }
            for q in u["query"].as_array().into_iter().flatten() {
                let k = text(q, "key");
                if k.is_empty() {
                    continue;
                }
                if req.query.insert(k.clone(), text(q, "value")).is_some() {
                    preview
                        .warnings
                        .push(format!("{name}: parámetro duplicado {k}"));
                }
                if q["disabled"] == true {
                    req.disabled_query.push(k);
                }
            }
        }
        let b = &r["body"];
        if b["disabled"] != true {
            req.body = match text(b, "mode").as_str() {
                "" => Body::None,
                "raw" => {
                    let raw = text(b, "raw");
                    match text(&b["options"]["raw"], "language").as_str() {
                        "json" => Body::Json { json: raw },
                        "xml" | "html" => Body::Xml { xml: raw },
                        _ => Body::Text { text: raw },
                    }
                }
                "urlencoded" => {
                    if b["urlencoded"]
                        .as_array()
                        .is_some_and(|fields| fields.iter().any(|field| field["disabled"] == true))
                    {
                        preview.warnings.push(format!(
                            "{name}: campos de formulario desactivados omitidos"
                        ));
                    }
                    Body::Form {
                        fields: fields(&b["urlencoded"]),
                    }
                }
                "file" => {
                    preview
                        .warnings
                        .push(format!("{name}: comprueba la ruta del archivo adjunto"));
                    Body::Binary {
                        path: text(&b["file"], "src"),
                    }
                }
                "formdata" => {
                    let mut parts = Vec::new();
                    for p in b["formdata"].as_array().into_iter().flatten() {
                        if p["disabled"] == true {
                            preview
                                .warnings
                                .push(format!("{name}: campo multipart desactivado omitido"));
                            continue;
                        }
                        let k = text(p, "key");
                        if text(p, "type") == "file" {
                            preview
                                .warnings
                                .push(format!("{name}: comprueba la ruta del archivo adjunto"));
                            parts.push(MultipartPart::File {
                                name: k,
                                path: text(p, "src"),
                                content_type: p["contentType"].as_str().map(str::to_owned),
                            });
                        } else {
                            parts.push(MultipartPart::Text {
                                name: k,
                                value: text(p, "value"),
                            });
                        }
                    }
                    Body::Multipart { parts }
                }
                mode => {
                    preview
                        .warnings
                        .push(format!("{name}: cuerpo {mode} no compatible"));
                    Body::None
                }
            };
        }
        req.scripts = scripts(&item["event"], &mut preview.warnings, name);
        req.docs = if r["description"].is_string() {
            text(r, "description")
        } else {
            text(&r["description"], "content")
        };
        if item["response"].as_array().is_some_and(|x| !x.is_empty()) {
            preview
                .warnings
                .push(format!("{name}: respuestas guardadas no importadas"));
        }
        preview.requests.push((key, req));
    }
    Ok(())
}
pub fn read(path: &Path) -> Result<Preview> {
    let meta = fs::metadata(path).map_err(|e| err(e.to_string()))?;
    if meta.len() > LIMIT {
        return Err(err("archivo Postman demasiado grande (máximo 20 MB)"));
    }
    let data = fs::read(path).map_err(|e| err(e.to_string()))?;
    let v: Value = serde_json::from_slice(&data)?;
    parse(&v)
}

pub(crate) fn parse(v: &Value) -> Result<Preview> {
    let schema = text(&v["info"], "schema");
    if !schema.contains("collection/v2.1.0/") {
        return Err(err("se requiere una colección Postman v2.1"));
    }
    let name = text(&v["info"], "name");
    if name.is_empty() {
        return Err(err("colección sin nombre"));
    }
    let mut p = Preview {
        name,
        warnings: Vec::new(),
        requests: Vec::new(),
        folders: Vec::new(),
        vars: fields(&v["variable"]),
        auth: Auth::None,
        scripts: Scripts::default(),
    };
    if v["variable"]
        .as_array()
        .is_some_and(|vars| vars.iter().any(|var| var["disabled"] == true))
    {
        p.warnings
            .push("Variables de colección desactivadas omitidas".into());
    }
    p.auth = auth(&v["auth"], &mut p.warnings, "Colección");
    if p.auth == Auth::Inherit {
        p.auth = Auth::None;
    }
    p.scripts = scripts(&v["event"], &mut p.warnings, "Colección");
    parse_items(&v["item"], "", &Auth::Inherit, &mut p, 0)?;
    Ok(p)
}
pub fn import(p: &Preview, target: &Path) -> Result<Collection> {
    fs::create_dir(target).map_err(|e| err(e.to_string()))?;
    let result = (|| {
        let mut c = Collection::init(target, &p.name)?;
        c.delete_request("health")?;
        c.delete_environment("dev")?;
        let mut meta = c.meta.clone();
        meta.default_environment = None;
        meta.vars.clone_from(&p.vars);
        meta.auth = p.auth.clone();
        meta.scripts = p.scripts.clone();
        c.save_meta(meta)?;
        for f in &p.folders {
            c.create_folder(f)?;
        }
        for (k, r) in &p.requests {
            c.save_request(k, r.clone())?;
        }
        Ok(c)
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(target);
    }
    result
}
fn post_auth(a: &Auth, warnings: &mut Vec<String>, context: &str) -> Value {
    let (kind, pairs): (&str, Vec<(&str, String)>) = match a {
        Auth::Inherit => return Value::Null,
        Auth::None => ("noauth", vec![]),
        Auth::Basic { username, password } => (
            "basic",
            vec![
                ("username", username.clone()),
                ("password", password.clone()),
            ],
        ),
        Auth::Bearer { token } => ("bearer", vec![("token", token.clone())]),
        Auth::Digest { username, password } => (
            "digest",
            vec![
                ("username", username.clone()),
                ("password", password.clone()),
            ],
        ),
        Auth::ApiKey { key, value, r#in } => (
            "apikey",
            vec![
                ("key", key.clone()),
                ("value", value.clone()),
                (
                    "in",
                    if *r#in == ApiKeyLocation::Query {
                        "query".into()
                    } else {
                        "header".into()
                    },
                ),
            ],
        ),
        Auth::OAuth2 { .. } => {
            warnings.push(format!("{context}: autenticación OAuth2 no exportada"));
            return json!({"type":"noauth"});
        }
    };
    let mut v = json!({"type":kind});
    if !pairs.is_empty() {
        v[kind] = json!(
            pairs
                .into_iter()
                .map(|(key, value)| json!({"key":key,"value":value,"type":"string"}))
                .collect::<Vec<_>>()
        );
    }
    v
}
fn events(s: &Scripts, warnings: &mut Vec<String>, context: &str) -> Vec<Value> {
    let mut out = Vec::new();
    if !s.pre.is_empty() {
        out.push(json!({"listen":"prerequest","script":{"type":"text/javascript","exec":s.pre.lines().collect::<Vec<_>>()}}));
    }
    if !s.test.is_empty() || !s.post.is_empty() {
        let combined = format!("{}\n{}", s.post, s.test);
        out.push(json!({"listen":"test","script":{"type":"text/javascript","exec":combined.lines().collect::<Vec<_>>()}}));
    }
    if !out.is_empty() {
        warnings.push(format!(
            "{context}: revisa la compatibilidad de los scripts en Postman"
        ));
    }
    out
}
fn post_request(r: &Request, warnings: &mut Vec<String>) -> Value {
    let (without_fragment, fragment) = r
        .url
        .split_once('#')
        .map_or((r.url.as_str(), None), |(u, f)| (u, Some(f)));
    let mut query: Vec<Value> = without_fragment
        .split_once('?')
        .map_or_else(Vec::new, |(_, q)| {
            url::form_urlencoded::parse(q.as_bytes())
                .map(|(k, v)| json!({"key":k,"value":v}))
                .collect()
        });
    query.extend(
        r.query
            .iter()
            .map(|(k, v)| json!({"key":k,"value":v,"disabled":r.disabled_query.contains(k)})),
    );
    let mut raw = without_fragment.to_owned();
    for (key, value) in &r.query {
        if r.disabled_query.contains(key) {
            continue;
        }
        raw.push(if raw.contains('?') { '&' } else { '?' });
        let encoded = url::form_urlencoded::Serializer::new(String::new())
            .append_pair(key, value)
            .finish()
            .replace("%7B", "{")
            .replace("%7D", "}");
        raw.push_str(&encoded);
    }
    if let Some(fragment) = fragment {
        raw.push('#');
        raw.push_str(fragment);
    }
    let headers: Vec<_> = r
        .headers
        .iter()
        .map(|(k, v)| json!({"key":k,"value":v,"type":"text"}))
        .collect();
    let body = match &r.body {
        Body::None => Value::Null,
        Body::Json { json: j } => {
            json!({"mode":"raw","raw":j,"options":{"raw":{"language":"json"}}})
        }
        Body::Xml { xml } => json!({"mode":"raw","raw":xml,"options":{"raw":{"language":"xml"}}}),
        Body::Text { text: t } => json!({"mode":"raw","raw":t}),
        Body::Binary { path } => {
            warnings.push(format!("{}: archivo adjunto no incluido", r.name));
            json!({"mode":"file","file":{"src":path}})
        }
        Body::Form { fields } => {
            json!({"mode":"urlencoded","urlencoded":fields.iter().map(|(k,v)| json!({"key":k,"value":v})).collect::<Vec<_>>()})
        }
        Body::Multipart { parts } => {
            json!({"mode":"formdata","formdata":parts.iter().map(|p| match p { MultipartPart::Text {name,value} => json!({"key":name,"value":value,"type":"text"}), MultipartPart::File {name,path,content_type} => { warnings.push(format!("{}: archivo adjunto no incluido",r.name)); json!({"key":name,"src":path,"type":"file","contentType":content_type}) } }).collect::<Vec<_>>()})
        }
    };
    if !r.expect.is_empty() {
        warnings.push(format!("{}: aserciones declarativas no exportadas", r.name));
    }
    let a = post_auth(&r.auth, warnings, &r.name);
    let mut req = json!({"method":r.method.to_string(),"header":headers,"url":{"raw":raw,"query":query},"description":r.docs});
    if !a.is_null() {
        req["auth"] = a;
    }
    if !body.is_null() {
        req["body"] = body;
    }
    json!({"name":r.name,"request":req,"event":events(&r.scripts,warnings,&r.name)})
}
fn insert_item(items: &mut Vec<Value>, segments: &[&str], value: Value) {
    if segments.is_empty() {
        items.push(value);
        return;
    }
    let name = segments[0];
    let index = items
        .iter()
        .position(|v| v["name"] == name && v["item"].is_array())
        .unwrap_or_else(|| {
            items.push(json!({"name":name,"item":[]}));
            items.len() - 1
        });
    insert_item(
        items[index]["item"].as_array_mut().expect("folder item"),
        &segments[1..],
        value,
    );
}
#[allow(clippy::missing_panics_doc)]
pub fn export(c: &Collection) -> (Value, Vec<String>) {
    let mut warnings = Vec::new();
    let mut items = Vec::new();
    for (key, r) in &c.requests {
        let seg: Vec<_> = key.split('/').collect();
        insert_item(
            &mut items,
            &seg[..seg.len() - 1],
            post_request(r, &mut warnings),
        );
    }
    let base = c.root.join("requests");
    for entry in walkdir::WalkDir::new(&base)
        .min_depth(1)
        .into_iter()
        .filter_map(std::result::Result::ok)
        .filter(|e| e.file_type().is_dir())
    {
        if let Ok(p) = entry.path().strip_prefix(&base) {
            let s = p.to_string_lossy().replace('\\', "/");
            let seg: Vec<_> = s.split('/').collect();
            let mut nodes = &mut items;
            for name in seg {
                let i = nodes
                    .iter()
                    .position(|v| v["name"] == name && v["item"].is_array())
                    .unwrap_or_else(|| {
                        nodes.push(json!({"name":name,"item":[]}));
                        nodes.len() - 1
                    });
                nodes = nodes[i]["item"].as_array_mut().expect("folder item");
            }
        }
    }
    if !c.folder_configs.is_empty() {
        warnings.push("Las variables de carpeta no se exportan a Postman".into());
    }
    if !c.environments.is_empty() {
        warnings.push("Los entornos no se incluyen en la colección Postman".into());
    }
    if c.meta.settings != crate::Settings::default() {
        warnings.push("Los ajustes de conexión no se exportan a Postman".into());
    }
    let variables: Vec<_> = c
        .meta
        .vars
        .iter()
        .map(|(k, v)| json!({"key":k,"value":v,"type":"string"}))
        .collect();
    let a = post_auth(&c.meta.auth, &mut warnings, "Colección");
    let e = events(&c.meta.scripts, &mut warnings, "Colección");
    let mut out = json!({"info":{"name":c.meta.name,"schema":SCHEMA},"item":items,"variable":variables,"event":e});
    if !a.is_null() {
        out["auth"] = a;
    }
    (out, warnings)
}
