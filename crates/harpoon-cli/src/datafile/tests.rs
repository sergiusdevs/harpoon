use super::*;

#[test]
fn csv_y_json() {
    let dir = tempfile::tempdir().unwrap();
    let csv_path = dir.path().join("d.csv");
    std::fs::write(&csv_path, "user,age\nada, 36\nlin,41\n").unwrap();
    let rows = load(&csv_path).unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["user"], "ada");
    assert_eq!(rows[0]["age"], "36");

    let json_path = dir.path().join("d.json");
    std::fs::write(&json_path, r#"[{"user":"ada","n":1,"tags":["a"]}]"#).unwrap();
    let rows = load(&json_path).unwrap();
    assert_eq!(rows[0]["n"], "1");
    assert_eq!(rows[0]["tags"], r#"["a"]"#);

    assert!(load(&dir.path().join("x.yaml")).is_err());
}
