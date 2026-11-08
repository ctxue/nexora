use windows::{
    core::PWSTR,
    Win32::{
        System::Com::{CoInitializeEx, CoTaskMemFree, CoUninitialize, COINIT_APARTMENTTHREADED},
        UI::Shell::{
            BHID_EnumItems, FOLDERID_AppsFolder, IEnumShellItems, IShellItem, SHGetKnownFolderItem,
            KNOWN_FOLDER_FLAG, SIGDN_DESKTOPABSOLUTEPARSING, SIGDN_NORMALDISPLAY,
        },
    },
};

unsafe fn display_name(
    item: &IShellItem,
    kind: windows::Win32::UI::Shell::SIGDN,
) -> Option<String> {
    let value: PWSTR = item.GetDisplayName(kind).ok()?;
    let text = value.to_string().ok();
    CoTaskMemFree(Some(value.0 as _));
    text
}

/// Enumerates the modern Windows "Apps" folder. Unlike the Start Menu, this
/// includes UWP/MSIX applications (for example Windows Terminal) and exposes
/// their localized display names.
pub fn scan_apps_folder() -> Vec<(String, String)> {
    unsafe {
        let initialized = CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_ok();

        let result = (|| {
            let folder: IShellItem =
                SHGetKnownFolderItem(&FOLDERID_AppsFolder, KNOWN_FOLDER_FLAG(0), None).ok()?;
            let enumerator: IEnumShellItems = folder.BindToHandler(None, &BHID_EnumItems).ok()?;

            let mut apps = Vec::new();
            loop {
                let mut items = [None; 1];
                let mut fetched = 0u32;
                enumerator.Next(&mut items, Some(&mut fetched)).ok()?;

                if fetched == 0 {
                    break;
                }

                let Some(item) = items[0].take() else {
                    continue;
                };
                let Some(name) = display_name(&item, SIGDN_NORMALDISPLAY) else {
                    continue;
                };
                let Some(parsing_name) = display_name(&item, SIGDN_DESKTOPABSOLUTEPARSING) else {
                    continue;
                };

                let name = name.trim().to_string();
                if name.is_empty() || parsing_name.trim().is_empty() {
                    continue;
                }

                let path = if parsing_name.contains(":\\") || parsing_name.starts_with("\\\\") {
                    parsing_name
                } else {
                    format!("shell:AppsFolder\\{parsing_name}")
                };

                apps.push((name, path));

                if apps.len() >= 1000 {
                    break;
                }
            }

            Some(apps)
        })()
        .unwrap_or_default();

        if initialized {
            CoUninitialize();
        }

        result
    }
}
