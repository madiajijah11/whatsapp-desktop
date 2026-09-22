// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Memory optimizations for glibc & WebKitGTK on Linux
    std::env::set_var("MALLOC_TRIM_THRESHOLD_", "131072");
    std::env::set_var("MALLOC_ARENA_MAX", "2");

    whatsapp_desktop_lib::run()
}
