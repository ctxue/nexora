use windows::{
    core::Result,
    Win32::{
        System::Com::{
            CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER,
            COINIT_MULTITHREADED,
        },
        UI::Accessibility::{
            CUIAutomation, IUIAutomation, IUIAutomationTextPattern, UIA_TextPatternId,
        },
    },
};

const MAX_SELECTION_CHARS: i32 = 8_192;

pub fn capture_focused_selection() -> String {
    unsafe {
        let initialized = CoInitializeEx(None, COINIT_MULTITHREADED).is_ok();
        let result: Result<String> = (|| {
            let automation: IUIAutomation =
                CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER)?;
            let focused = automation.GetFocusedElement()?;
            let pattern: IUIAutomationTextPattern =
                focused.GetCurrentPatternAs(UIA_TextPatternId)?;
            let ranges = pattern.GetSelection()?;
            if ranges.Length()? <= 0 {
                return Ok(String::new());
            }
            let range = ranges.GetElement(0)?;
            let text = range.GetText(MAX_SELECTION_CHARS)?;
            Ok(text.to_string())
        })();
        if initialized {
            CoUninitialize();
        }
        result.unwrap_or_default()
    }
}
