use super::*;

#[test]
fn digest_rfc7616_ejemplo_md5() {
    // Ejemplo de la RFC 7616 §3.9.1 (sin cnonce fija no podemos comparar
    // el response; comprobamos la estructura y el algoritmo).
    let h = r#"Digest realm="http-auth@example.org", qop="auth, auth-int", algorithm=MD5, nonce="7ypf/xlj9XXwfDPEoM4URrv/xwf94BcCAzFZH4GiTo0v", opaque="FQhe/qaU925kfnzjCev0ciny7QMkPqMAFRtzCUYo5tdS""#;
    let a = digest_authorization(h, "GET", "/dir/index.html", "Mufasa", "Circle of Life").unwrap();
    assert!(a.starts_with("Digest username=\"Mufasa\", realm=\"http-auth@example.org\""));
    assert!(a.contains("qop=auth, nc=00000001"));
    assert!(a.contains("opaque=\"FQhe/qaU925kfnzjCev0ciny7QMkPqMAFRtzCUYo5tdS\""));
    assert!(a.contains("algorithm=MD5"));
}

#[test]
fn digest_sin_qop_es_determinista() {
    // RFC 2617 §3.5 (sin qop): response conocida.
    let h = r#"Digest realm="testrealm@host.com", nonce="dcd98b7102dd2f0e8b11d0f600bfb0c093", opaque="5ccc069c403ebaf9f0171e9517f40e41""#;
    let a = digest_authorization(h, "GET", "/dir/index.html", "Mufasa", "Circle Of Life").unwrap();
    assert!(
        a.contains("response=\"670fd8c2df070c60b045671b8b24ff02\""),
        "{a}"
    );
}

#[test]
fn uri_de_url() {
    assert_eq!(request_uri("https://h.com/a/b?c=1"), "/a/b?c=1");
    assert_eq!(request_uri("https://h.com"), "/");
}
