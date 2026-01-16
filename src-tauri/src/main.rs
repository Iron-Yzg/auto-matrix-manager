// 主入口点 - Tauri 2.0 应用入口
// Main entry point for Tauri 2.0 application

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    auto_matrix_manager::run();
}
