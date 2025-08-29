// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri_plugin_store::StoreBuilder;

fn main() {
    secure_file_sharing_system_lib::run()
}
