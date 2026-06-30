// ============================================================
// mole/settings.rs —— 用户配置的持久化存储模块
// 负责将用户自定义的 Mole CLI 路径保存到本地文件中，
// 并在需要时读取出来。底层使用 tauri-plugin-store 插件，
// 数据存储在 settings.json 文件里（类似 Java 的 Properties 文件）
// ============================================================

// 引入标准库的 PathBuf 类型，用于表示文件系统路径
// PathBuf 是可变的路径，Path 是不可变的路径引用（类似 Java 的 File vs Path 接口）
use std::path::PathBuf;

// 引入 tauri-plugin-store 的扩展 trait，用于通过 app 实例访问持久化存储
// StoreExt 是一个 trait（类似 Java 的接口），给 AppHandle 附加了 .store() 方法
use tauri_plugin_store::StoreExt;

// 存储文件的名称（相对于应用数据目录，由 Tauri 自动管理路径）
// const 是编译时常量，类似 Java 的 static final
const STORE_PATH: &str = "settings.json";

// 在 JSON 存储中，Mole CLI 路径对应的键名
// &str 是字符串切片，是 Rust 中最基础的字符串类型（只读引用）
const MOLE_PATH_KEY: &str = "mole_path";

/// 从持久化存储中读取用户配置的 Mole CLI 路径。
///
/// 参数:
///   app - Tauri 应用句柄（相当于 Spring 的 ApplicationContext，可以获取各种服务）
///
/// 返回值:
///   Option<PathBuf> —— Some(路径) 表示找到了有效路径，None 表示没有配置或路径不存在
///   Option 是 Rust 的空值安全类型，相当于 Java 的 Optional<File>
pub fn get_configured_mole_path(app: &tauri::AppHandle) -> Option<PathBuf> {
    // 打开持久化存储文件（settings.json）
    // .ok()? 的含义：
    //   - .ok() 将 Result<T,E> 转换为 Option<T>（错误时变成 None）
    //   - ? 是"提前返回"运算符：如果是 None，立即从函数返回 None
    //   相当于 Java 的：if (store == null) return null;
    let store = app.store(STORE_PATH).ok()?;

    // 从存储中读取键为 "mole_path" 的值
    // store.get() 返回 Option<serde_json::Value>（JSON 值）
    let value = store.get(MOLE_PATH_KEY)?;

    // 将 JSON 值转换为字符串引用 &str
    // as_str() 返回 Option<&str>，如果不是字符串类型则返回 None
    let path_str = value.as_str()?;

    // 如果路径字符串为空，返回 None（没有配置路径）
    if path_str.is_empty() {
        return None;
    }

    // 将字符串转换为 PathBuf（路径对象）
    let path = PathBuf::from(path_str);

    // 检查路径是否真实存在于文件系统上
    if path.exists() {
        // 规范化路径（解析软链接、修正大小写等）
        // 在 macOS 大小写不敏感的文件系统上，这很重要
        // canonicalize() 可能失败（比如路径包含符号链接但链接目标不存在）
        // .ok() 将 Result 转为 Option
        // .or(Some(path)) 表示：如果规范化失败，就直接用原始路径
        // 相当于 Java 的：return canonicalized != null ? canonicalized : path;
        path.canonicalize().ok().or(Some(path))
    } else {
        // 路径不存在（可能是用户之前配置了但后来删除了该程序），返回 None
        None
    }
}

/// 将用户配置的 Mole CLI 路径保存到持久化存储中。
///
/// 参数:
///   app  - Tauri 应用句柄
///   path - 要保存的路径字符串；传空字符串表示清除自定义配置，恢复自动检测
///
/// 返回值:
///   Result<(), String> —— Ok(()) 表示成功，Err(错误信息字符串) 表示失败
///   相当于 Java 的 void 方法，但会抛出 Exception
pub fn set_configured_mole_path(app: &tauri::AppHandle, path: &str) -> Result<(), String> {
    // 打开存储文件，如果打开失败则返回错误信息
    // map_err 将 tauri 的错误类型转换为 String 类型
    // 相当于 Java 的 catch(Exception e) { throw new RuntimeException("...", e); }
    let store = app
        .store(STORE_PATH)
        .map_err(|e| format!("Failed to open settings store: {}", e))?;

    // 将路径字符串存入 JSON 存储
    // serde_json::Value::String(...) 将 Rust String 包装成 JSON 字符串类型
    // to_string() 将 &str 转换为拥有所有权的 String（相当于 Java 的 new String(str)）
    store.set(MOLE_PATH_KEY.to_string(), serde_json::Value::String(path.to_string()));

    // 将内存中的更改持久化到磁盘（写入 settings.json 文件）
    // 如果写入失败，返回错误信息
    store
        .save()
        .map_err(|e| format!("Failed to persist settings: {}", e))?;

    // Ok(()) 表示函数成功执行，没有需要返回的数据
    // 相当于 Java void 方法正常返回
    Ok(())
}
