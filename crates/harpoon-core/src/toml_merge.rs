//! Reescritura de TOML que conserva lo que escribió una persona: comentarios,
//! orden de las claves, comillas y formato de los valores que no cambian.
//!
//! En vez de serializar de cero, se aplica el documento nuevo sobre el que ya
//! hay en disco: las claves que siguen igual no se tocan, las que cambian
//! cambian de valor (manteniendo su comentario de línea), las que desaparecen
//! se quitan y las nuevas se añaden al final de su tabla.

use toml_edit::{DocumentMut, InlineTable, Item, Table, Value};

/// Aplica `new_text` sobre `old_text`. Devuelve `None` si alguno no es TOML
/// válido (el llamador escribe entonces `new_text` tal cual).
#[must_use]
pub fn merge(old_text: &str, new_text: &str) -> Option<String> {
    let mut old: DocumentMut = old_text.parse().ok()?;
    let new: DocumentMut = new_text.parse().ok()?;
    merge_table(old.as_table_mut(), new.as_table());
    Some(old.to_string())
}

fn merge_table(old: &mut Table, new: &Table) {
    let gone: Vec<String> = old
        .iter()
        .map(|(k, _)| k.to_owned())
        .filter(|k| !new.contains_key(k))
        .collect();
    for key in gone {
        old.remove(&key);
    }
    for (key, item) in new {
        if let Some(current) = old.get_mut(key) {
            merge_item(current, item);
        } else {
            let mut item = item.clone();
            forget_positions(&mut item);
            old.insert(key, item);
        }
    }
}

fn merge_item(old: &mut Item, new: &Item) {
    match (&mut *old, new) {
        (Item::Table(o), Item::Table(n)) => merge_table(o, n),
        (Item::Value(o), Item::Value(n)) => merge_value(o, n),
        // Quien escribió `x = { … }` a mano conserva la tabla en línea.
        (Item::Value(Value::InlineTable(o)), Item::Table(n)) => {
            merge_inline(o, &n.clone().into_inline_table());
        }
        _ => {
            let mut item = new.clone();
            forget_positions(&mut item);
            *old = item;
        }
    }
}

fn merge_value(old: &mut Value, new: &Value) {
    if same(old, new) {
        return;
    }
    if let (Value::InlineTable(o), Value::InlineTable(n)) = (&mut *old, new) {
        merge_inline(o, n);
        return;
    }
    let decor = old.decor().clone();
    *old = new.clone();
    *old.decor_mut() = decor;
}

fn merge_inline(old: &mut InlineTable, new: &InlineTable) {
    let gone: Vec<String> = old
        .iter()
        .map(|(k, _)| k.to_owned())
        .filter(|k| !new.contains_key(k))
        .collect();
    for key in gone {
        old.remove(&key);
    }
    for (key, value) in new {
        match old.get_mut(key) {
            Some(current) => merge_value(current, value),
            None => {
                old.insert(key, value.clone());
            }
        }
    }
}

/// Mismo valor TOML aunque se haya escrito distinto (`'a'` y `"a"`, `1_000` y `1000`).
fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::String(x), Value::String(y)) => x.value() == y.value(),
        (Value::Integer(x), Value::Integer(y)) => x.value() == y.value(),
        (Value::Float(x), Value::Float(y)) => x.value().to_bits() == y.value().to_bits(),
        (Value::Boolean(x), Value::Boolean(y)) => x.value() == y.value(),
        (Value::Datetime(x), Value::Datetime(y)) => x.value() == y.value(),
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y.iter()).all(|(a, b)| same(a, b))
        }
        (Value::InlineTable(x), Value::InlineTable(y)) => {
            x.len() == y.len() && x.iter().all(|(k, v)| y.get(k).is_some_and(|w| same(v, w)))
        }
        _ => false,
    }
}

/// Las tablas copiadas del documento nuevo traen su posición en él; sin
/// posición se colocan detrás de las existentes.
fn forget_positions(item: &mut Item) {
    match item {
        Item::Table(t) => {
            t.set_position(None);
            for (_, child) in t.iter_mut() {
                forget_positions(child);
            }
        }
        Item::ArrayOfTables(a) => {
            for t in a.iter_mut() {
                t.set_position(None);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::merge;

    #[test]
    fn conserva_comentarios_orden_y_comillas() {
        let old = r#"# Listado público de usuarios
name = 'Listar usuarios'   # nombre visible
method = "GET"
url = "{{base_url}}/users" # sin barra final

[headers]
# Necesario para la API v2
Accept = "application/json"
X-Old = "1"
"#;
        let new = r#"name = "Listar usuarios"
method = "POST"
url = "{{base_url}}/users"

[headers]
Accept = "application/json"
X-New = "2"
"#;
        let out = merge(old, new).unwrap();
        assert_eq!(
            out,
            r#"# Listado público de usuarios
name = 'Listar usuarios'   # nombre visible
method = "POST"
url = "{{base_url}}/users" # sin barra final

[headers]
# Necesario para la API v2
Accept = "application/json"
X-New = "2"
"#
        );
    }

    #[test]
    fn tablas_nuevas_al_final_y_quitar_tablas() {
        let old = "name = \"a\"\n\n# auth de prueba\n[auth]\ntype = \"none\"\n\n[body]\ntype = \"none\"\n";
        let new =
            "name = \"a\"\n\n[body]\ntype = \"json\"\njson = \"{}\"\n\n[expect]\nstatus = 200\n";
        let out = merge(old, new).unwrap();
        assert_eq!(
            out,
            "name = \"a\"\n\n[body]\ntype = \"json\"\njson = \"{}\"\n\n[expect]\nstatus = 200\n"
        );
    }

    #[test]
    fn respeta_tablas_en_linea_escritas_a_mano() {
        let old = "vars = { a = \"1\", b = \"2\" } # comunes\n";
        let new = "[vars]\na = \"1\"\nb = \"3\"\n";
        assert_eq!(
            merge(old, new).unwrap(),
            "vars = { a = \"1\", b = \"3\" } # comunes\n"
        );
    }

    #[test]
    fn sin_cambios_el_archivo_queda_identico() {
        let old = "# hola\nname = 'x'  # fin\n\n[headers]\nA = \"1\"\n";
        let new = "name = \"x\"\n\n[headers]\nA = \"1\"\n";
        assert_eq!(merge(old, new).unwrap(), old);
    }

    #[test]
    fn toml_invalido_devuelve_none() {
        assert_eq!(merge("esto no es = = toml", "a = 1"), None);
    }
}
