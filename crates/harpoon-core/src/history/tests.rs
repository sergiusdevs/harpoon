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
    assert!(load(tmp.path(), "big").is_empty());
}
