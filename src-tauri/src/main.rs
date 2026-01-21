// 主入口点 - Tauri 2.0 应用入口
// Main entry point for Tauri 2.0 application

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // 注意：Tauri 内部已经初始化了 tracing，无需再次初始化
    // 日志会通过 Tauri 的日志系统自动输出
    auto_matrix_manager::run();
}
