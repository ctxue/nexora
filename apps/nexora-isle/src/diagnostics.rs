use std::path::{Path, PathBuf};

fn write_report(directory: &Path, title: &str, details: &str) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(directory)?;
    let path = directory.join("last-error.log");
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    std::fs::write(
        &path,
        format!(
            "NEXORA ISLE {}\n系统：{} / {}\n时间（Unix 秒）：{timestamp}\n{title}\n{details}\n",
            env!("CARGO_PKG_VERSION"),
            std::env::consts::OS,
            std::env::consts::ARCH,
        ),
    )?;
    Ok(path)
}

fn message(directory: &Path, title: &str, details: &str) -> String {
    match write_report(directory, title, details) {
        Ok(path) => format!(
            "{title}\n\n请将以下日志发给开发者，帮助定位问题：\n{}",
            path.display()
        ),
        Err(error) => {
            format!("{title}\n\n{details}\n\n日志无法保存：{error}\n请保存现有日志文件后反馈。")
        }
    }
}

pub fn report(title: &str, details: &str) {
    let text = message(
        &nexora_isle::model::config_dir().join("diagnostics"),
        title,
        details,
    );
    #[cfg(windows)]
    {
        use windows::core::{w, PCWSTR};
        use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};
        let wide: Vec<u16> = text.encode_utf16().chain(Some(0)).collect();
        unsafe {
            let mut centering = nexora_isle::system::NativeDialogCenter::start(None);
            MessageBoxW(
                None,
                PCWSTR(wide.as_ptr()),
                w!("NEXORA ISLE"),
                MB_OK | MB_ICONERROR,
            );
            centering.finish();
        }
    }
    #[cfg(not(windows))]
    eprintln!("{text}");
}

pub fn startup(stage: &str) {
    let directory = nexora_isle::model::config_dir().join("diagnostics");
    if std::fs::create_dir_all(&directory).is_ok() {
        let _ = std::fs::write(
            directory.join("last-startup.log"),
            format!(
                "NEXORA ISLE {}\n进程：{}\n启动阶段：{stage}\n",
                env!("CARGO_PKG_VERSION"),
                std::process::id()
            ),
        );
    }
}

pub fn graphics(cc: &eframe::CreationContext<'_>) {
    use eframe::glow::HasContext;
    let details = if let Some(gl) = &cc.gl {
        unsafe {
            format!(
                "厂商：{}\n渲染器：{}\nOpenGL：{}\n",
                gl.get_parameter_string(eframe::glow::VENDOR),
                gl.get_parameter_string(eframe::glow::RENDERER),
                gl.get_parameter_string(eframe::glow::VERSION)
            )
        }
    } else if let Some(state) = &cc.wgpu_render_state {
        let info = state.adapter.get_info();
        format!(
            "渲染器：{}\n图形后端：{:?}\n设备类型：{:?}\n",
            info.name, info.backend, info.device_type
        )
    } else {
        return;
    };
    let directory = nexora_isle::model::config_dir().join("diagnostics");
    if std::fs::create_dir_all(&directory).is_ok() {
        let _ = std::fs::write(directory.join("last-graphics.log"), details);
    }
}

pub fn install() {
    std::panic::set_hook(Box::new(|info| {
        let location = info
            .location()
            .map(|p| format!("{}:{}:{}", p.file(), p.line(), p.column()))
            .unwrap_or_else(|| "未知".into());
        report(
            "NEXORA ISLE 发生异常，请重新启动。",
            &format!(
                "代码位置：{location}\n调用栈：\n{}",
                std::backtrace::Backtrace::force_capture(),
            ),
        );
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn directory(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "nexora-isle-diagnostics-{}-{name}",
            std::process::id()
        ))
    }

    #[test]
    fn failure_creates_a_report_and_exposes_its_location() {
        let directory = directory("report");
        let text = message(&directory, "启动失败", "OpenGL 初始化失败");
        let path = directory.join("last-error.log");
        assert!(text.contains(&path.display().to_string()));
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains(env!("CARGO_PKG_VERSION")));
        assert!(content.contains("OpenGL 初始化失败"));
        write_report(&directory, "第二次失败", "新的错误").unwrap();
        assert!(!std::fs::read_to_string(&path).unwrap().contains("OpenGL"));
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }

    #[test]
    fn unwritable_log_keeps_the_error_visible() {
        let path = directory("blocked");
        std::fs::write(&path, b"test").unwrap();
        let text = message(&path, "启动失败", "OpenGL 初始化失败");
        assert!(text.contains("OpenGL 初始化失败"));
        assert!(text.contains("日志无法保存"));
        std::fs::remove_file(path).unwrap();
    }
}
