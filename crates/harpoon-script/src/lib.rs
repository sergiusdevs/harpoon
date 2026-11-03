//! `harpoon-script`: motor JavaScript embebido (`QuickJS`) y pipeline de
//! ejecución completo de una solicitud: scripts `pre`, envío HTTP, scripts
//! `post`, `test` y aserciones `[expect]`.
//!
//! Los scripts no tienen acceso a red, disco ni al sistema: solo ven el
//! objeto `harpoon` (request, response, variables) y utilidades de test.

pub mod engine;
pub mod runner;

pub use engine::{Engine, Error as ScriptError, ScriptRequest, TestResult};
pub use runner::{Execution, execute};
