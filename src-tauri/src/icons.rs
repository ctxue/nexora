use std::{io::Cursor, path::Path};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use file_icon_provider::get_file_icon;
use serde::Serialize;

const ICON_REQUEST_SIZE: u16 = 256;
const ICON_OUTPUT_SIZE: u32 = 64;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IconPayload {
    pub path: String,
    pub icon: Option<String>,
}

pub fn icons_for_paths(paths: Vec<String>) -> Vec<IconPayload> {
    paths
        .into_iter()
        .take(32)
        .map(|path| {
            let icon = icon_for_path(&path);
            IconPayload { path, icon }
        })
        .collect()
}

fn trim_icon_padding(source: &image::RgbaImage) -> image::RgbaImage {
    let (width, height) = source.dimensions();
    let mut min_x = width;
    let mut min_y = height;
    let mut max_x = 0;
    let mut max_y = 0;
    let mut found = false;

    for y in 0..height {
        for x in 0..width {
            let pixel = source.get_pixel(x, y);
            let transparent = pixel[3] < 64;
            let near_white = pixel[0] > 238 && pixel[1] > 238 && pixel[2] > 238;

            if transparent || near_white {
                continue;
            }

            found = true;
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }
    }

    if !found {
        return source.clone();
    }

    let crop_width = max_x - min_x + 1;
    let crop_height = max_y - min_y + 1;

    // Keep the original image when the shell already returned a tightly
    // cropped icon. Otherwise a white 256px canvas would leave the real icon
    // tiny in the middle of the row.
    if crop_width < 4 || crop_height < 4 || (crop_width * crop_height) > (width * height * 95 / 100)
    {
        return source.clone();
    }

    image::imageops::crop_imm(source, min_x, min_y, crop_width, crop_height).to_image()
}

fn icon_for_path(path: &str) -> Option<String> {
    if path.trim().is_empty() {
        return None;
    }

    if path.starts_with("shell:AppsFolder\\") {
        let source = crate::shell_icon::get_shell_icon(path, ICON_REQUEST_SIZE)?;
        return encode_icon(source);
    }

    if !Path::new(path).exists() {
        return None;
    }

    let icon = get_file_icon(path, ICON_REQUEST_SIZE).ok()?;
    let source = image::RgbaImage::from_raw(icon.width, icon.height, icon.pixels)?;
    encode_icon(source)
}

fn encode_icon(source: image::RgbaImage) -> Option<String> {
    let cropped = trim_icon_padding(&source);

    let resized = image::imageops::resize(
        &cropped,
        ICON_OUTPUT_SIZE,
        ICON_OUTPUT_SIZE,
        image::imageops::FilterType::Lanczos3,
    );

    let mut png = Cursor::new(Vec::new());
    resized.write_to(&mut png, image::ImageFormat::Png).ok()?;

    Some(format!(
        "data:image/png;base64,{}",
        STANDARD.encode(png.into_inner())
    ))
}
