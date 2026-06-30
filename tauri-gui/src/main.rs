// 条件编译属性：在非 debug 模式（即发布模式）下，将程序设置为 Windows 子系统。
// 这样在 Windows 上运行时不会弹出黑色的命令行窗口，只显示 GUI 界面。
// 类似 Java 里的 -Djavaw 参数，macOS 上无影响，可以忽略。
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// main 函数是 Rust 程序的入口点，相当于 Java 的 public static void main(String[] args)
fn main() {
    // 调用我们库（lib.rs）里定义的 run() 函数，启动整个 Tauri 应用
    // mole_gui_lib 是 Cargo.toml 里配置的库名称
    mole_gui_lib::run()
}
