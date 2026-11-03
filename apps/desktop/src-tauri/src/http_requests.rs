//! Cancelación de envíos HTTP y errores estables para la interfaz.
use serde::Serialize;
use std::{
    collections::HashMap,
    future::Future,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::sync::watch;

#[derive(Debug, Clone, Serialize)]
pub struct Failure {
    pub code: &'static str,
    pub title: &'static str,
    pub message: &'static str,
    pub detail: String,
}

impl Failure {
    pub fn cancelled() -> Self {
        Self {
            code: "cancelled",
            title: "Solicitud cancelada",
            message: "El envío se ha interrumpido.",
            detail: String::new(),
        }
    }
    pub fn from_detail(detail: impl Into<String>) -> Self {
        let detail = detail.into();
        let lower = detail.to_lowercase();
        let (code, title, message) = if lower.contains("variable sin definir") {
            (
                "variable",
                "Variable sin resolver",
                "Define la variable en la colección, carpeta o entorno activo.",
            )
        } else if ["certificate", "certificado", "tls", "invalid peer"]
            .iter()
            .any(|part| lower.contains(part))
        {
            (
                "tls",
                "Error de certificado o TLS",
                "Revisa la CA, el nombre del servidor y los certificados configurados.",
            )
        } else if ["timeout", "timed out", "tiempo agotado"]
            .iter()
            .any(|part| lower.contains(part))
        {
            (
                "timeout",
                "Tiempo de espera agotado",
                "Comprueba el servidor o ajusta el timeout de la colección.",
            )
        } else if [
            "connection refused",
            "conexión rechazada",
            "dns",
            "failed to lookup",
            "error trying to connect",
            "network is unreachable",
        ]
        .iter()
        .any(|part| lower.contains(part))
        {
            (
                "connection",
                "No se pudo conectar",
                "Comprueba que el servidor esté activo y que la URL, el puerto y el proxy sean correctos.",
            )
        } else if ["oauth2", "digest", "unauthorized", "autenticación"]
            .iter()
            .any(|part| lower.contains(part))
        {
            (
                "authentication",
                "Error de autenticación",
                "Revisa las credenciales, el token y la configuración de autenticación.",
            )
        } else {
            (
                "request",
                "No se pudo enviar",
                "Revisa los detalles de la solicitud y la configuración de la colección.",
            )
        };
        Self {
            code,
            title,
            message,
            detail,
        }
    }
}
impl From<harpoon_script::runner::Error> for Failure {
    fn from(error: harpoon_script::runner::Error) -> Self {
        Self::from_detail(error.to_string())
    }
}

struct Entry {
    signal: watch::Sender<bool>,
    started: bool,
    created: Instant,
}
#[derive(Clone, Default)]
pub struct Requests(Arc<Mutex<HashMap<String, Entry>>>);
struct Registration {
    id: String,
    requests: Requests,
}
impl Drop for Registration {
    fn drop(&mut self) {
        if let Ok(mut entries) = self.requests.0.lock() {
            entries.remove(&self.id);
        }
    }
}
impl Requests {
    fn entry<'a>(
        entries: &'a mut HashMap<String, Entry>,
        id: &str,
    ) -> Result<&'a mut Entry, Failure> {
        if id.is_empty() || id.len() > 100 {
            return Err(Failure::from_detail("Identificador de envío no válido"));
        }
        entries
            .retain(|_, entry| entry.started || entry.created.elapsed() < Duration::from_secs(60));
        if entries.len() >= 256 && !entries.contains_key(id) {
            return Err(Failure::from_detail("Demasiados envíos pendientes"));
        }
        Ok(entries.entry(id.to_owned()).or_insert_with(|| Entry {
            signal: watch::channel(false).0,
            started: false,
            created: Instant::now(),
        }))
    }
    pub fn cancel(&self, id: &str) -> Result<(), Failure> {
        let mut entries = self
            .0
            .lock()
            .map_err(|_| Failure::from_detail("Registro de envíos no disponible"))?;
        Self::entry(&mut entries, id)?.signal.send_replace(true);
        Ok(())
    }
    pub async fn run<T>(
        &self,
        id: &str,
        future: impl Future<Output = Result<T, Failure>>,
    ) -> Result<T, Failure> {
        let mut cancelled = {
            let mut entries = self
                .0
                .lock()
                .map_err(|_| Failure::from_detail("Registro de envíos no disponible"))?;
            let entry = Self::entry(&mut entries, id)?;
            if entry.started {
                return Err(Failure::from_detail(
                    "El identificador de envío ya está en uso",
                ));
            }
            entry.started = true;
            entry.signal.subscribe()
        };
        let _registration = Registration {
            id: id.to_owned(),
            requests: self.clone(),
        };
        if *cancelled.borrow() {
            return Err(Failure::cancelled());
        }
        tokio::select! {
            biased;
            _ = cancelled.changed() => Err(Failure::cancelled()),
            result = future => result,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn cancela_antes_de_registrar_sin_ejecutar_el_envio() {
        let requests = Requests::default();
        requests.cancel("one").unwrap();
        let result: Result<(), Failure> = requests
            .run("one", async {
                panic!("no debe ejecutarse");
            })
            .await;
        assert_eq!(result.unwrap_err().code, "cancelled");
        assert!(requests.0.lock().unwrap().is_empty());
    }
    #[tokio::test]
    async fn cancela_un_envio_en_curso_y_limpia_el_registro() {
        let requests = Requests::default();
        let other = requests.clone();
        let task = tokio::spawn(async move {
            other
                .run("one", std::future::pending::<Result<(), Failure>>())
                .await
        });
        tokio::task::yield_now().await;
        requests.cancel("one").unwrap();
        assert_eq!(task.await.unwrap().unwrap_err().code, "cancelled");
        assert!(requests.0.lock().unwrap().is_empty());
    }
    #[tokio::test]
    async fn terminar_un_envio_no_cancela_otro() {
        let requests = Requests::default();
        assert_eq!(requests.run("one", async { Ok(42) }).await.unwrap(), 42);
        assert!(requests.0.lock().unwrap().is_empty());
        assert_eq!(requests.run("two", async { Ok(43) }).await.unwrap(), 43);
    }
    #[test]
    fn distingue_las_causas_de_error() {
        for (detail, code) in [
            ("variable sin definir: token", "variable"),
            ("invalid peer certificate", "tls"),
            ("connection refused", "connection"),
            ("tiempo agotado tras 30s", "timeout"),
            ("OAuth2: invalid client", "authentication"),
        ] {
            assert_eq!(Failure::from_detail(detail).code, code);
        }
    }
}
