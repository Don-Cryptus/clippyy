use super::hotkey::hotkey_listener::init_hotkey_listener;
use crate::types::types::Key;
use crate::{
    service::window::get_data_path, types::types::Config,
    utils::clipboard::clipboard_handler::Handler,
};
use arboard::Clipboard;
use clipboard_master::Master;
use global_hotkey::GlobalHotKeyManager;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::{fs, path::Path, sync::OnceLock};
use tauri::{LogicalSize, Manager};
// use window_shadows::set_shadow;

pub static MAIN_WINDOW_X: i32 = 375;
pub static MAIN_WINDOW_Y: i32 = 600;

pub static APP: OnceLock<tauri::AppHandle> = OnceLock::new();
pub static HOTKEY_MANAGER: OnceLock<GlobalHotKeyManager> = OnceLock::new();
pub static HOTKEYS: OnceLock<Arc<Mutex<HashMap<u32, Key>>>> = OnceLock::new();
pub static CLIPBOARD: OnceLock<Arc<Mutex<Clipboard>>> = OnceLock::new();

pub fn setup(app: &mut tauri::App) -> Result<(), Box<(dyn std::error::Error + 'static)>> {
    APP.set(app.handle()).expect("error initializing tauri app");
    let _ = HOTKEY_MANAGER.set(GlobalHotKeyManager::new().unwrap());
    let _ = HOTKEYS.set(Arc::new(Mutex::new(HashMap::new())));
    let _ = CLIPBOARD.set(Arc::new(Mutex::new(Clipboard::new()?)));

    create_config();

    let window = app.get_window("main").unwrap();

    let _ = window.set_size(LogicalSize::new(MAIN_WINDOW_X, MAIN_WINDOW_Y));

    #[cfg(any(windows, target_os = "macos"))]
    set_shadow(&window, true).unwrap();

    #[cfg(debug_assertions)]
    {
        window.open_devtools();
    }

    tauri::async_runtime::spawn(async { Master::new(Handler).run() });
    // tauri::async_runtime::spawn(async { init_hotkey_listener() });

    init_hotkey_listener();

    Ok(())
}

pub fn create_config() {
    let data_path = get_data_path();

    if Path::new(&data_path.config_file_path).exists() {
        return;
    }

    let config = Config {
        db: format!("{}", &data_path.db_file_path),
    };

    let _ = fs::write(
        &data_path.config_file_path,
        serde_json::to_string(&config).unwrap(),
    );
}
