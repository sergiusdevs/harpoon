//! Estado del servidor mock local de la aplicación.
use serde::Serialize;
use std::path::PathBuf;
use tauri::State;
use tokio::sync::Mutex;

#[derive(Default)]
pub struct MockState(Mutex<Option<Running>>);
struct Running {
    root: PathBuf,
    server: harpoon_mock::Server,
}
#[derive(Serialize)]
pub struct MockStatus {
    root: PathBuf,
    address: String,
}
#[tauri::command]
pub fn load_mock(root: PathBuf) -> Result<harpoon_mock::Config, String> {
    let col = harpoon_core::Collection::discover(&root).map_err(|e| e.to_string())?;
    harpoon_mock::load(&col.root)
}
#[tauri::command]
pub async fn save_mock(
    root: PathBuf,
    config: harpoon_mock::Config,
    state: State<'_, MockState>,
) -> Result<(), String> {
    let col = harpoon_core::Collection::discover(&root).map_err(|e| e.to_string())?;
    let current = state.0.lock().await;
    if current
        .as_ref()
        .is_some_and(|r| r.root == col.root && !r.server.finished())
    {
        return Err("Detén el servidor antes de guardar los cambios".into());
    }
    harpoon_mock::save(&col.root, &config)
}
#[tauri::command]
pub async fn mock_status(state: State<'_, MockState>) -> Result<Option<MockStatus>, String> {
    let mut current = state.0.lock().await;
    if current.as_ref().is_some_and(|r| r.server.finished()) {
        *current = None;
    }
    Ok(current.as_ref().map(|running| MockStatus {
        root: running.root.clone(),
        address: format!("http://{}", running.server.address),
    }))
}
#[tauri::command]
pub async fn start_mock(
    root: PathBuf,
    port: u16,
    state: State<'_, MockState>,
) -> Result<MockStatus, String> {
    let col = harpoon_core::Collection::discover(&root).map_err(|e| e.to_string())?;
    let mut current = state.0.lock().await;
    if current.as_ref().is_some_and(|r| !r.server.finished()) {
        return Err("Ya hay un servidor mock en ejecución".into());
    }
    let config = harpoon_mock::load(&col.root)?;
    let server = harpoon_mock::start(config, port).await?;
    let status = MockStatus {
        root: col.root.clone(),
        address: format!("http://{}", server.address),
    };
    *current = Some(Running {
        root: col.root,
        server,
    });
    Ok(status)
}
#[tauri::command]
pub async fn stop_mock(state: State<'_, MockState>) -> Result<(), String> {
    let mut current = state.0.lock().await;
    if let Some(running) = current.take() {
        running.server.stop().await?;
    }
    Ok(())
}
