// ============================================================
// 开机自启动管理（Windows 注册表）
// 替换 electron/task-scheduler-autostart.js
// ============================================================

#[cfg(target_os = "windows")]
pub mod windows {
    const REG_KEY: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Run";
    const APP_NAME: &str = "SyncClassroomStudent";

    pub fn get_autostart() -> bool {
        let hkcu = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
        hkcu.open_subkey(REG_KEY)
            .and_then(|key| key.get_value::<String, _>(APP_NAME))
            .is_ok()
    }

    pub fn set_autostart(enable: bool, exe_path: &str) -> Result<(), String> {
        let hkcu = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
        let key = hkcu
            .open_subkey_with_flags(REG_KEY, winreg::enums::KEY_WRITE)
            .map_err(|e| e.to_string())?;

        if enable {
            key.set_value(APP_NAME, &exe_path)
                .map_err(|e| e.to_string())
        } else {
            // 键不存在时 delete_value 会返回错误，忽略即可
            let _ = key.delete_value(APP_NAME);
            Ok(())
        }
    }
}

// macOS / Linux 占位（可按需扩展）
#[cfg(not(target_os = "windows"))]
pub mod windows {
    pub fn get_autostart() -> bool { false }
    pub fn set_autostart(_enable: bool, _exe: &str) -> Result<(), String> { Ok(()) }
}
