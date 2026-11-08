#[cfg(not(windows))]
pub(crate) fn system_proxy() -> Option<String> {
    None
}

#[cfg(windows)]
pub(crate) fn system_proxy() -> Option<String> {
    use windows::{
        core::w,
        Win32::System::Registry::{
            RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RRF_RT_REG_SZ,
        },
    };

    let key = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings");
    let mut enabled = 0u32;
    let mut size = 4u32;
    unsafe {
        if RegGetValueW(
            HKEY_CURRENT_USER,
            key,
            w!("ProxyEnable"),
            RRF_RT_REG_DWORD,
            None,
            Some((&mut enabled as *mut u32).cast()),
            Some(&mut size),
        )
        .is_err()
            || enabled == 0
        {
            return None;
        }
    }

    let mut buffer = [0u16; 1024];
    size = (buffer.len() * 2) as u32;
    unsafe {
        if RegGetValueW(
            HKEY_CURRENT_USER,
            key,
            w!("ProxyServer"),
            RRF_RT_REG_SZ,
            None,
            Some(buffer.as_mut_ptr().cast()),
            Some(&mut size),
        )
        .is_err()
        {
            return None;
        }
    }

    let text = String::from_utf16_lossy(
        &buffer[..buffer.iter().position(|v| *v == 0).unwrap_or(buffer.len())],
    );
    let server = if text.contains('=') {
        text.split(';')
            .find_map(|value| value.trim().strip_prefix("https="))
            .or_else(|| {
                text.split(';')
                    .find_map(|value| value.trim().strip_prefix("http="))
            })?
    } else {
        text.trim()
    };
    if server.is_empty() {
        None
    } else if server.contains("://") {
        Some(server.into())
    } else {
        Some(format!("http://{server}"))
    }
}
