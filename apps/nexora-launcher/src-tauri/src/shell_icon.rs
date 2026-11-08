use std::{ffi::c_void, mem::size_of};

use windows::{
    core::HSTRING,
    Win32::{
        Foundation::SIZE,
        Graphics::Gdi::{
            CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits, GetObjectW, BITMAP, BITMAPINFO,
            BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
        },
        System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED},
        UI::Shell::{
            IShellItemImageFactory, SHCreateItemFromParsingName, SIIGBF_ICONONLY, SIIGBF_SCALEUP,
        },
    },
};

pub fn get_shell_icon(parsing_name: &str, size: u16) -> Option<image::RgbaImage> {
    unsafe {
        let initialized = CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_ok();

        let result = (|| {
            let factory: IShellItemImageFactory =
                SHCreateItemFromParsingName(&HSTRING::from(parsing_name), None).ok()?;
            let hbitmap = factory
                .GetImage(
                    SIZE {
                        cx: i32::from(size),
                        cy: i32::from(size),
                    },
                    SIIGBF_ICONONLY | SIIGBF_SCALEUP,
                )
                .ok()?;

            hbitmap_to_image(hbitmap)
        })();

        if initialized {
            CoUninitialize();
        }

        result
    }
}

unsafe fn hbitmap_to_image(
    hbitmap: windows::Win32::Graphics::Gdi::HBITMAP,
) -> Option<image::RgbaImage> {
    let mut bitmap: BITMAP = std::mem::zeroed();
    if GetObjectW(
        hbitmap.into(),
        size_of::<BITMAP>() as i32,
        Some((&mut bitmap as *mut BITMAP).cast::<c_void>()),
    ) == 0
    {
        let _ = DeleteObject(hbitmap.into());
        return None;
    }

    let width = u32::try_from(bitmap.bmWidth).ok()?;
    let height = u32::try_from(bitmap.bmHeight).ok()?;
    if width == 0 || height == 0 {
        let _ = DeleteObject(hbitmap.into());
        return None;
    }

    let mut bitmap_info: BITMAPINFO = std::mem::zeroed();
    bitmap_info.bmiHeader.biSize = size_of::<BITMAPINFOHEADER>() as u32;
    bitmap_info.bmiHeader.biWidth = bitmap.bmWidth;
    bitmap_info.bmiHeader.biHeight = -bitmap.bmHeight;
    bitmap_info.bmiHeader.biPlanes = 1;
    bitmap_info.bmiHeader.biBitCount = 32;
    bitmap_info.bmiHeader.biCompression = BI_RGB.0;

    let mut pixels = vec![0u8; width as usize * height as usize * 4];
    let hdc = CreateCompatibleDC(None);
    if hdc.is_invalid() {
        let _ = DeleteObject(hbitmap.into());
        return None;
    }

    let copied = GetDIBits(
        hdc,
        hbitmap,
        0,
        height,
        Some(pixels.as_mut_ptr().cast()),
        &mut bitmap_info,
        DIB_RGB_COLORS,
    );
    let _ = DeleteDC(hdc);
    let _ = DeleteObject(hbitmap.into());

    if copied == 0 {
        return None;
    }

    // GetDIBits returns BGRA; the renderer expects RGBA.
    for pixel in pixels.as_chunks_mut::<4>().0 {
        pixel.swap(0, 2);
    }

    image::RgbaImage::from_raw(width, height, pixels)
}
