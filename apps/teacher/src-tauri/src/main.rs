// Windows 下隐藏控制台窗口（release 模式）
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    syncclassroom_teacher_lib::run()
}
