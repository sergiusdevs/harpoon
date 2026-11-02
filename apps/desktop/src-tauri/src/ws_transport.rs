//! Transporte WebSocket con los ajustes de TLS y proxy de la colección.
use harpoon_core::Collection;
use hyper_util::client::proxy::matcher::{Intercept, Matcher};
use rustls::{
    ClientConfig, DigitallySignedStruct, RootCertStore, SignatureScheme,
    client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
    crypto::{CryptoProvider, verify_tls12_signature, verify_tls13_signature},
    pki_types::{CertificateDer, ServerName, UnixTime},
};
use std::{
    io::{BufReader, Cursor},
    sync::Arc,
    time::Duration,
};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    net::TcpStream,
};
use tokio_rustls::TlsConnector;
use tokio_tungstenite::{
    Connector, MaybeTlsStream, WebSocketStream, client_async_tls_with_config,
    tungstenite::{self, http::Request, protocol::WebSocketConfig},
};

pub(crate) trait AsyncIo: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> AsyncIo for T {}
type Io = Box<dyn AsyncIo>;
pub type Socket = WebSocketStream<MaybeTlsStream<Io>>;
#[derive(Debug)]
pub enum ConnectError {
    Transport(String),
    Handshake(tungstenite::Error),
}
impl std::fmt::Display for ConnectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transport(e) => f.write_str(e),
            Self::Handshake(e) => write!(f, "{e}"),
        }
    }
}

#[derive(Debug)]
struct InsecureVerifier(Arc<CryptoProvider>);
impl ServerCertVerifier for InsecureVerifier {
    fn verify_server_cert(
        &self,
        _: &CertificateDer<'_>,
        _: &[CertificateDer<'_>],
        _: &ServerName<'_>,
        _: &[u8],
        _: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }
    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        sig: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls12_signature(
            message,
            cert,
            sig,
            &self.0.signature_verification_algorithms,
        )
    }
    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        sig: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls13_signature(
            message,
            cert,
            sig,
            &self.0.signature_verification_algorithms,
        )
    }
    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.0.signature_verification_algorithms.supported_schemes()
    }
}
fn tls_config(collection: &Collection) -> Result<Arc<ClientConfig>, String> {
    let settings = &collection.meta.settings;
    let provider = Arc::new(rustls::crypto::aws_lc_rs::default_provider());
    let builder = ClientConfig::builder_with_provider(provider.clone())
        .with_safe_default_protocol_versions()
        .map_err(|e| e.to_string())?;
    let builder = if settings.insecure {
        builder
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(InsecureVerifier(provider)))
    } else {
        let mut roots = RootCertStore::empty();
        roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        if let Some(path) = &settings.ca_cert {
            let data =
                std::fs::read(collection.root.join(path)).map_err(|e| format!("CA {path}: {e}"))?;
            let certs = rustls_pemfile::certs(&mut Cursor::new(data))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;
            if certs.is_empty() {
                return Err(format!("CA {path}: no contiene certificados PEM"));
            }
            for cert in certs {
                roots.add(cert).map_err(|e| e.to_string())?;
            }
        }
        builder.with_root_certificates(roots)
    };
    let config = if let Some(path) = &settings.client_cert {
        let cert_data = std::fs::read(collection.root.join(path))
            .map_err(|e| format!("Certificado {path}: {e}"))?;
        let certs = rustls_pemfile::certs(&mut Cursor::new(&cert_data))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        let key_data = if let Some(key) = &settings.client_key {
            std::fs::read(collection.root.join(key)).map_err(|e| format!("Clave {key}: {e}"))?
        } else {
            cert_data
        };
        let key = rustls_pemfile::private_key(&mut BufReader::new(Cursor::new(key_data)))
            .map_err(|e| e.to_string())?
            .ok_or("No se encontró una clave privada PEM")?;
        builder
            .with_client_auth_cert(certs, key)
            .map_err(|e| format!("Certificado de cliente: {e}"))?
    } else {
        builder.with_no_client_auth()
    };
    Ok(Arc::new(config))
}
fn proxy_for(collection: &Collection, target: &url::Url) -> Result<Option<Intercept>, String> {
    let mut http_target = target.clone();
    let scheme = if target.scheme() == "wss" {
        "https"
    } else {
        "http"
    };
    http_target
        .set_scheme(scheme)
        .map_err(|()| "URL de destino no válida")?;
    let uri: tungstenite::http::Uri = http_target
        .as_str()
        .parse()
        .map_err(|_| "URL de destino no válida")?;
    if let Some(proxy) = &collection.meta.settings.proxy {
        let parsed = url::Url::parse(proxy).map_err(|_| "URL del proxy no válida")?;
        if !matches!(parsed.scheme(), "http" | "https") {
            return Err("El proxy debe usar http o https".into());
        }
        let found = Matcher::builder()
            .all(proxy.as_str())
            .build()
            .intercept(&uri);
        return found
            .map(Some)
            .ok_or_else(|| "No se pudo configurar el proxy".into());
    }
    Ok(Matcher::from_system().intercept(&uri))
}
async fn transport(
    collection: &Collection,
    target: &url::Url,
    tls: Arc<ClientConfig>,
) -> Result<Io, String> {
    let host = target.host_str().ok_or("La URL no tiene host")?;
    let port = target
        .port_or_known_default()
        .ok_or("La URL no tiene puerto")?;
    let authority = if host.contains(':') && !host.starts_with('[') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    };
    let Some(proxy) = proxy_for(collection, target)? else {
        return Ok(Box::new(
            TcpStream::connect((host.trim_matches(['[', ']']), port))
                .await
                .map_err(|e| format!("Conexión: {e}"))?,
        ));
    };
    let uri = proxy.uri();
    let proxy_host = uri.host().ok_or("Proxy sin host")?.trim_matches(['[', ']']);
    let proxy_scheme = uri.scheme_str().ok_or("Proxy sin esquema")?;
    if !matches!(proxy_scheme, "http" | "https") {
        return Err("El proxy debe usar http o https".into());
    }
    let proxy_port = uri
        .port_u16()
        .unwrap_or(if proxy_scheme == "https" { 443 } else { 80 });
    let tcp = TcpStream::connect((proxy_host, proxy_port))
        .await
        .map_err(|e| format!("Conexión al proxy: {e}"))?;
    let mut io: Io = if proxy_scheme == "https" {
        let name =
            ServerName::try_from(proxy_host.to_owned()).map_err(|_| "Host del proxy no válido")?;
        Box::new(
            TlsConnector::from(tls)
                .connect(name, tcp)
                .await
                .map_err(|e| format!("TLS del proxy: {e}"))?,
        )
    } else {
        Box::new(tcp)
    };
    let mut connect = format!("CONNECT {authority} HTTP/1.1\r\nHost: {authority}\r\n");
    if let Some(auth) = proxy.basic_auth() {
        let auth = auth
            .to_str()
            .map_err(|_| "Autenticación de proxy no válida")?;
        connect.push_str("Proxy-Authorization: ");
        connect.push_str(auth);
        connect.push_str("\r\n");
    }
    connect.push_str("\r\n");
    io.write_all(connect.as_bytes())
        .await
        .map_err(|e| format!("Proxy: {e}"))?;
    io.flush().await.map_err(|e| format!("Proxy: {e}"))?;
    let mut response = Vec::new();
    let mut byte = [0u8; 1];
    while !response.ends_with(b"\r\n\r\n") {
        if response.len() >= 16384 {
            return Err("Respuesta del proxy demasiado grande".into());
        }
        io.read_exact(&mut byte)
            .await
            .map_err(|e| format!("Proxy: {e}"))?;
        response.push(byte[0]);
    }
    let line = response.split(|b| *b == b'\n').next().unwrap_or_default();
    let line = std::str::from_utf8(line).map_err(|_| "Respuesta del proxy no válida")?;
    let mut words = line.split_whitespace();
    let version = words.next().unwrap_or("");
    let status = words.next().unwrap_or("");
    if !version.starts_with("HTTP/1.") || status != "200" {
        return Err(format!("El proxy rechazó CONNECT (HTTP {status})"));
    }
    Ok(io)
}
pub async fn connect(
    collection: &Collection,
    target: &url::Url,
    request: Request<()>,
    config: WebSocketConfig,
) -> Result<Socket, ConnectError> {
    let run = async {
        let tls = tls_config(collection).map_err(ConnectError::Transport)?;
        let io = transport(collection, target, tls.clone())
            .await
            .map_err(ConnectError::Transport)?;
        let (socket, _) =
            client_async_tls_with_config(request, io, Some(config), Some(Connector::Rustls(tls)))
                .await
                .map_err(ConnectError::Handshake)?;
        Ok(socket)
    };
    let timeout = collection.meta.settings.timeout_ms;
    if timeout == 0 {
        run.await
    } else {
        tokio::time::timeout(Duration::from_millis(timeout), run)
            .await
            .map_err(|_| ConnectError::Transport("Tiempo de conexión agotado".into()))?
    }
}
