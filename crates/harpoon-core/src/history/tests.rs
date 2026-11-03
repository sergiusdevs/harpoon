use super::*;

fn entry(status: u16) -> HistoryEntry {
    HistoryEntry {
        at: now_iso(),
        env: Some("dev".into()),
        method: "GET".into(),
        url: "http://x/".into(),
        status,
        elapsed_ms: 3,
        size: 2,
        headers: IndexMap::new(),
        body: "{}".into(),
        truncated: false,
        checks_passed: 1,
        checks_total: 1,
    }
}

#[test]
fn append_recorta_y_carga() {
    let tmp = tempfile::tempdir().unwrap();
    for i in 0..(MAX_ENTRIES + 5) {
        append(
            tmp.path(),
            "users/list",
            entry(u16::try_from(200 + i).unwrap()),
        )
        .unwrap();
    }
    let all = load(tmp.path(), "users/list");
    assert_eq!(all.len(), MAX_ENTRIES);
    assert_eq!(
        all.last().unwrap().status,
        200 + u16::try_from(MAX_ENTRIES + 4).unwrap()
    );
    assert!(
        tmp.path()
            .join(".harpoon/history/users__list.jsonl")
            .is_file()
    );

    let mut big = entry(200);
    big.body = "x".repeat(MAX_BODY + 10);
    append(tmp.path(), "big", big).unwrap();
    let b = &load(tmp.path(), "big")[0];
    assert!(b.truncated);
    assert_eq!(b.body.len(), MAX_BODY);

    clear(tmp.path(), "big").unwrap();
    assert_eq!(load(tmp.path(), "big"), vec![]);
}

#[test]
fn el_historial_no_guarda_credenciales() {
    let tmp = tempfile::tempdir().unwrap();
    let mut e = entry(200);
    e.url = "https://api.x.com/a?page=2&access_token=abc123&api_key=zzz#top".into();
    e.headers = IndexMap::from([
        ("Set-Cookie".to_string(), "session=s3cr3t".to_string()),
        ("Content-Type".to_string(), "application/json".to_string()),
        ("X-Auth-Token".to_string(), "t0k3n".to_string()),
    ]);
    append(tmp.path(), "a", e).unwrap();
    let saved = &load(tmp.path(), "a")[0];
    assert_eq!(
        saved.url,
        format!("https://api.x.com/a?page=2&access_token={HIDDEN}&api_key={HIDDEN}#top")
    );
    assert_eq!(saved.headers["Set-Cookie"], HIDDEN);
    assert_eq!(saved.headers["X-Auth-Token"], HIDDEN);
    assert_eq!(saved.headers["Content-Type"], "application/json");
    let raw =
        std::fs::read_to_string(tmp.path().join(".harpoon/history/a.jsonl")).unwrap_or_default();
    assert!(!raw.contains("s3cr3t") && !raw.contains("abc123"));
}
