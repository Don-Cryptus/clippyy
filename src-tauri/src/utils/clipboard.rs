extern crate alloc;
use crate::service::clipboard::{parse_model, upsert};
use clipboard_master::{CallbackResult, ClipboardHandler};
use std::io;
use tauri::Manager;

use super::setup::APP;

pub struct Handler;

impl ClipboardHandler for Handler {
    fn on_clipboard_change(&mut self) -> CallbackResult {
        // let res = tokio::task::block_in_place(|| {
        //     tokio::runtime::Handle::current().block_on(async move {
        //         let model = parse_model();
        //         insert(model).await
        //     })
        // });
        println!("Clipboard changed");

        let _ = tauri::async_runtime::spawn(async {
            let model = parse_model();

            let model = upsert(model).await.unwrap();
            // let main_window = APP.get_window("main").unwrap();
            let main_window = APP.get().unwrap().get_window("main").unwrap();
            main_window.emit("clipboard_listener", model).unwrap();
        });

        CallbackResult::Next
    }

    fn on_clipboard_error(&mut self, error: io::Error) -> CallbackResult {
        println!("Error: {}", error);
        CallbackResult::Next
    }
}
