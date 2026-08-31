// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Memory & cache optimizations for WebKitGTK on Linux
    std::env::set_var("MALLOC_TRIM_THRESHOLD_", "131072");
    std::env::set_var("G_SLICE", "always-malloc");
    std::env::set_var("WEBKIT_MEMORY_PRESSURE_RELIEF_PERCENT", "75");

    whatsapp_desktop_lib::run()
}
