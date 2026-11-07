use eframe::egui::{
    self, pos2, vec2, Color32, Context, Id, Rect, SizeHint, TextureHandle, TextureOptions,
};
use egui_extras::image::load_svg_bytes_with_size;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub(crate) enum Icon {
    Settings,
    LayoutGrid,
    Sparkles,
    Music,
    Bell,
    Info,
    Languages,
    Monitor,
    Sun,
    Moon,
    GripVertical,
    Clipboard,
    Image,
    PlayCircle,
    Activity,
    Cpu,
    MemoryStick,
    HardDrive,
    Gauge,
    Folder,
    Close,
    ChevronLeft,
    ChevronRight,
    ChevronUp,
    ChevronDown,
    Pin,
    PinFilled,
    SkipBack,
    SkipForward,
    Archive,
    Bluetooth,
    Headphones,
    Speaker,
    VolumeOn,
    VolumeOff,
    Keyboard,
    Mouse,
    Gamepad,
    Smartphone,
}

impl Icon {
    fn bytes(self) -> &'static [u8] {
        match self {
            Self::Settings => include_bytes!("../assets/lucide/settings-2.svg"),
            Self::LayoutGrid => include_bytes!("../assets/lucide/layout-grid.svg"),
            Self::Sparkles => include_bytes!("../assets/lucide/sparkles.svg"),
            Self::Music => include_bytes!("../assets/lucide/music-2.svg"),
            Self::Bell => include_bytes!("../assets/lucide/bell-ring.svg"),
            Self::Info => include_bytes!("../assets/lucide/info.svg"),
            Self::Languages => include_bytes!("../assets/lucide/languages.svg"),
            Self::Monitor => include_bytes!("../assets/lucide/monitor.svg"),
            Self::Sun => include_bytes!("../assets/lucide/sun.svg"),
            Self::Moon => include_bytes!("../assets/lucide/moon.svg"),
            Self::GripVertical => include_bytes!("../assets/lucide/grip-vertical.svg"),
            Self::Clipboard => include_bytes!("../assets/lucide/clipboard.svg"),
            Self::Image => include_bytes!("../assets/lucide/image.svg"),
            Self::PlayCircle => include_bytes!("../assets/lucide/play-circle.svg"),
            Self::Activity => include_bytes!("../assets/lucide/activity.svg"),
            Self::Cpu => include_bytes!("../assets/lucide/cpu.svg"),
            Self::MemoryStick => include_bytes!("../assets/lucide/memory-stick.svg"),
            Self::HardDrive => include_bytes!("../assets/lucide/hard-drive.svg"),
            Self::Gauge => include_bytes!("../assets/lucide/gauge.svg"),
            Self::Folder => include_bytes!("../assets/lucide/folder.svg"),
            Self::Close => include_bytes!("../assets/lucide/x.svg"),
            Self::ChevronLeft => include_bytes!("../assets/lucide/chevron-left.svg"),
            Self::ChevronRight => include_bytes!("../assets/lucide/chevron-right.svg"),
            Self::ChevronUp => include_bytes!("../assets/lucide/chevron-up.svg"),
            Self::ChevronDown => include_bytes!("../assets/lucide/chevron-down.svg"),
            Self::Pin => include_bytes!("../assets/lucide/pin.svg"),
            Self::PinFilled => include_bytes!("../assets/lucide/pin-filled.svg"),
            Self::SkipBack => include_bytes!("../assets/lucide/skip-back.svg"),
            Self::SkipForward => include_bytes!("../assets/lucide/skip-forward.svg"),
            Self::Archive => include_bytes!("../assets/lucide/archive.svg"),
            Self::Bluetooth => include_bytes!("../assets/lucide/bluetooth.svg"),
            Self::Headphones => include_bytes!("../assets/lucide/headphones.svg"),
            Self::Speaker => include_bytes!("../assets/lucide/speaker.svg"),
            Self::VolumeOn => include_bytes!("../assets/lucide/volume-2.svg"),
            Self::VolumeOff => include_bytes!("../assets/lucide/volume-x.svg"),
            Self::Keyboard => include_bytes!("../assets/lucide/keyboard.svg"),
            Self::Mouse => include_bytes!("../assets/lucide/mouse.svg"),
            Self::Gamepad => include_bytes!("../assets/lucide/gamepad-2.svg"),
            Self::Smartphone => include_bytes!("../assets/lucide/smartphone.svg"),
        }
    }

    fn texture(self, ctx: &Context) -> TextureHandle {
        let id = Id::new(("lucide-texture", self));
        if let Some(texture) = ctx.data(|data| data.get_temp::<TextureHandle>(id)) {
            return texture;
        }
        let image = load_svg_bytes_with_size(self.bytes(), Some(SizeHint::Height(64)))
            .expect("embedded Lucide SVG must be valid");
        let texture = ctx.load_texture(format!("lucide-{self:?}"), image, TextureOptions::LINEAR);
        ctx.data_mut(|data| data.insert_temp(id, texture.clone()));
        texture
    }
}

pub(crate) fn paint(painter: &egui::Painter, rect: Rect, icon: Icon, color: Color32) {
    painter.image(
        icon.texture(painter.ctx()).id(),
        rect,
        Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
        color,
    );
}

pub(crate) fn image(ctx: &Context, icon: Icon, size: f32, color: Color32) -> egui::Image<'static> {
    let texture = icon.texture(ctx);
    egui::Image::new(egui::load::SizedTexture::new(
        texture.id(),
        vec2(size, size),
    ))
    .fit_to_exact_size(vec2(size, size))
    .tint(color)
}

pub(crate) fn card(
    card: &crate::paint::Card<'_>,
    icon: Icon,
    cx: f32,
    cy: f32,
    size: f32,
    color: Color32,
) {
    paint(
        &card.painter,
        card.rect(cx - size / 2.0, cy - size / 2.0, size, size),
        icon,
        color,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_lucide_icons_render_with_visible_strokes() {
        let icons = [
            Icon::Settings,
            Icon::LayoutGrid,
            Icon::Sparkles,
            Icon::Music,
            Icon::Bell,
            Icon::Info,
            Icon::Languages,
            Icon::Monitor,
            Icon::Sun,
            Icon::Moon,
            Icon::GripVertical,
            Icon::Clipboard,
            Icon::Image,
            Icon::PlayCircle,
            Icon::Activity,
            Icon::Cpu,
            Icon::MemoryStick,
            Icon::HardDrive,
            Icon::Gauge,
            Icon::Folder,
            Icon::Close,
            Icon::ChevronLeft,
            Icon::ChevronRight,
            Icon::ChevronUp,
            Icon::ChevronDown,
            Icon::Pin,
            Icon::PinFilled,
            Icon::SkipBack,
            Icon::SkipForward,
            Icon::Archive,
            Icon::Bluetooth,
            Icon::Headphones,
            Icon::Speaker,
            Icon::VolumeOn,
            Icon::VolumeOff,
            Icon::Keyboard,
            Icon::Mouse,
            Icon::Gamepad,
            Icon::Smartphone,
        ];
        for icon in icons {
            let image = load_svg_bytes_with_size(icon.bytes(), Some(SizeHint::Height(64)))
                .unwrap_or_else(|error| panic!("{icon:?}: {error}"));
            assert!(image.pixels.iter().any(|pixel| pixel.a() > 0), "{icon:?}");
        }
    }
}
