// Copyright 2019-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

// use tauri::{
//     AppHandle, CustomMenuItem, GlobalShortcutManager, GlobalWindowEvent, Manager, SystemTray,
//     SystemTrayEvent, SystemTrayMenu, WindowEvent,
// };

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// use tauri::AppHandle;
// use tauri::GlobalShortcutManager;
// use tauri::Manager;

#[tauri::command]
async fn call_me(app_handle: tauri::AppHandle) {
  use tauri::Manager;
  let window = app_handle.get_window("main").unwrap();
  window.open_devtools();
}

fn main() {
  tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![call_me,])
    .run(tauri::generate_context!(
      "../../examples/helloworld/tauri.conf.json"
    ))
    .expect("error while running tauri application");
}
