use enigo::{Enigo, MouseControllable};
use tauri::{Manager, PhysicalPosition};

use crate::utils::setup::APP;

#[tauri::command]
pub async fn window_on_mouse() -> Result<(), String> {
    let win = APP.get().unwrap().get_window("main").unwrap();
    let enigo = Enigo::new();
    let (x, y) = enigo.mouse_location();

    let _ = win.set_position(PhysicalPosition::new(x, y));
    // Ok(res.unwrap())
    Ok(())
}
