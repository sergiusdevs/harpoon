use super::*;

fn layer(pairs: &[(&str, &str)]) -> IndexMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect()
}

#[test]
fn interpola_y_respeta_prioridad() {
    let scope = Scope::new()
        .with_layer(layer(&[("base_url", "http://col"), ("v", "1")]))
        .with_layer(layer(&[("base_url", "http://env")]));
    assert_eq!(
        scope.interpolate("{{base_url}}/api/v{{ v }}").unwrap(),
        "http://env/api/v1"
    );
}

#[test]
fn variable_sin_definir_es_error() {
    let scope = Scope::new();
    assert!(matches!(
        scope.interpolate("{{nope}}"),
        Err(Error::UndefinedVariable(n)) if n == "nope"
    ));
}

#[test]
fn variables_dinamicas() {
    let scope = Scope::new();
    let u = scope.interpolate("{{$uuid}}").unwrap();
    assert_eq!(u.len(), 36);
    let n: i64 = scope
        .interpolate("{{$randomInt 5 7}}")
        .unwrap()
        .parse()
        .unwrap();
    assert!((5..=7).contains(&n));
    assert_eq!(scope.interpolate("{{$randomHex 8}}").unwrap().len(), 8);
    let ts: u64 = scope
        .interpolate("{{$timestamp}}")
        .unwrap()
        .parse()
        .unwrap();
    assert!(ts > 1_700_000_000);
    let iso = scope.interpolate("{{$isoTimestamp}}").unwrap();
    assert!(iso.ends_with('Z') && iso.len() == 20, "{iso}");
    // Una variable `$desconocida` sigue siendo un error.
    assert!(scope.interpolate("{{$nope}}").is_err());
}

#[test]
fn llaves_sin_cerrar_quedan_literales() {
    let scope = Scope::new();
    assert_eq!(scope.interpolate("a {{ b").unwrap(), "a {{ b");
}
