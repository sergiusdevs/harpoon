//! `harpoon-core`: el núcleo de Harpoon.
//!
//! Contiene el modelo de datos (colecciones, solicitudes, entornos), la
//! lectura/escritura del formato de colección en disco (archivos TOML
//! legibles y aptos para Git) y el motor de variables (`{{nombre}}`).
//!
//! Este crate no hace red ni ejecuta scripts: eso vive en crates separados
//! (`harpoon-http`, `harpoon-script`) para que el núcleo sea pequeño, puro y
//! fácil de probar.

pub mod collection;
pub mod error;
pub mod history;
pub mod model;
pub mod openapi;
pub mod postman;
pub mod share;
pub mod vars;

pub use collection::Collection;
pub use error::Error;
pub use model::{Auth, Body, Environment, Expect, Method, Request, RequestType, Settings};
pub use vars::Scope;

/// Resultado estándar del crate.
pub type Result<T> = std::result::Result<T, Error>;
