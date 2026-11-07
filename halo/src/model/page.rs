//! Retained NEXORA HALO pages and legacy settings migration.

pub const MUSIC: usize = 0;
pub const SHELF: usize = 1;
pub const CLIPBOARD: usize = 2;
pub const MAIL: usize = 3;
pub const IMAGES: usize = 4;
pub const VIDEO: usize = 5;
pub const MONITOR: usize = 6;
pub const COUNT: usize = 7;
pub const ALL: [usize; COUNT] = [MUSIC, VIDEO, MONITOR, SHELF, IMAGES, CLIPBOARD, MAIL];

pub fn retained_name(index: usize) -> &'static str {
    match index {
        MUSIC => "音乐",
        VIDEO => "视频",
        MONITOR => "监控",
        SHELF => "文件口袋",
        IMAGES => "图片",
        CLIPBOARD => "剪贴板",
        MAIL => "邮箱",
        _ => "未知",
    }
}

pub fn retained_name_for(index: usize, language: crate::i18n::Language) -> &'static str {
    match index {
        MUSIC => language.text("音乐", "Music"),
        VIDEO => language.text("视频", "Video"),
        MONITOR => language.text("监控", "Monitor"),
        SHELF => language.text("文件口袋", "Pocket"),
        IMAGES => language.text("图片", "Images"),
        CLIPBOARD => language.text("剪贴板", "Clipboard"),
        MAIL => language.text("邮箱", "Mail"),
        _ => language.text("未知", "Unknown"),
    }
}

pub fn from_legacy(index: usize) -> Option<usize> {
    match index {
        0 => Some(MUSIC),
        3 => Some(SHELF),
        5 => Some(MAIL),
        _ => None,
    }
}

/// The previous four-page release used music, shelf, tasks, and mail. Tasks were
/// removed, so migrate its positional settings without leaving a hole.
pub fn migrate_v4_order(legacy: [usize; 4]) -> [usize; COUNT] {
    let map = |index| match index {
        0 => Some(MUSIC),
        1 => Some(SHELF),
        3 => Some(MAIL),
        _ => None,
    };
    let mut order = Vec::with_capacity(COUNT);
    for page in legacy.into_iter().filter_map(map) {
        if !order.contains(&page) {
            order.push(page);
        }
    }
    finish_order(order)
}

pub fn migrate_v4_visibility(raw: &[u8]) -> [bool; COUNT] {
    let mut visible = [false; COUNT];
    for legacy in [MUSIC, SHELF, 3] {
        let mapped = match legacy {
            MUSIC => MUSIC,
            SHELF => SHELF,
            _ => MAIL,
        };
        visible[mapped] = raw.get(legacy) == Some(&b'1');
    }
    visible[CLIPBOARD] = visible[SHELF];
    visible[IMAGES] = visible[SHELF];
    visible[VIDEO] = visible[MUSIC];
    visible[MONITOR] = true;
    if !visible.contains(&true) {
        visible[MUSIC] = true;
    }
    visible
}

pub fn migrate_v3_order(legacy: [usize; 3]) -> [usize; COUNT] {
    let map = |index| match index {
        MUSIC => Some(MUSIC),
        SHELF => Some(SHELF),
        _ => Some(MAIL),
    };
    let mut order = Vec::with_capacity(COUNT);
    for page in legacy.into_iter().filter_map(map) {
        if !order.contains(&page) {
            order.push(page);
        }
    }
    // Keep the brand-new Clipboard page next to Pocket on first migration.
    if let (Some(shelf), Some(clipboard)) = (
        order.iter().position(|page| *page == SHELF),
        order.iter().position(|page| *page == CLIPBOARD),
    ) {
        let clipboard_page = order.remove(clipboard);
        order.insert(shelf + 1, clipboard_page);
    }
    finish_order(order)
}

pub fn migrate_v3_visibility(raw: &[u8]) -> [bool; COUNT] {
    let mut visible = [false; COUNT];
    visible[MUSIC] = raw.first() == Some(&b'1');
    visible[SHELF] = raw.get(1) == Some(&b'1');
    visible[CLIPBOARD] = visible[SHELF];
    visible[IMAGES] = visible[SHELF];
    visible[MAIL] = raw.get(2) == Some(&b'1');
    visible[VIDEO] = visible[MUSIC];
    visible[MONITOR] = true;
    if !visible.contains(&true) {
        visible[MUSIC] = true;
    }
    visible
}

pub fn migrate_legacy_order<const N: usize>(legacy: [usize; N]) -> [usize; COUNT] {
    let mut order = Vec::with_capacity(COUNT);
    for index in legacy {
        if let Some(page) = from_legacy(index) {
            if !order.contains(&page) {
                order.push(page);
            }
        }
    }
    finish_order(order)
}

pub fn migrate_legacy_visibility(raw: &[u8]) -> [bool; COUNT] {
    let mut visible = [false; COUNT];
    for (legacy, enabled) in raw.iter().copied().enumerate() {
        if enabled != b'1' {
            continue;
        }
        if let Some(page) = from_legacy(legacy) {
            visible[page] = true;
        }
    }
    visible[CLIPBOARD] = visible[SHELF];
    visible[IMAGES] = visible[SHELF];
    visible[VIDEO] = visible[MUSIC];
    visible[MONITOR] = true;
    if !visible.contains(&true) {
        visible[MUSIC] = true;
    }
    visible
}

/// Previous releases saved exactly four live pages. Preserve that order and
/// insert the two new media categories next to their related pages.
pub fn migrate_current_v4_order(legacy: [usize; 4]) -> [usize; COUNT] {
    let mut order = Vec::with_capacity(COUNT);
    for page in legacy {
        if page < IMAGES && !order.contains(&page) {
            order.push(page);
        }
    }
    if let Some(music) = order.iter().position(|page| *page == MUSIC) {
        order.insert(music + 1, VIDEO);
    }
    if let Some(shelf) = order.iter().position(|page| *page == SHELF) {
        order.insert(shelf + 1, IMAGES);
    }
    finish_order(order)
}

pub fn migrate_current_v4_visibility(raw: &[u8]) -> [bool; COUNT] {
    let mut visible = [false; COUNT];
    for (page, enabled) in visible.iter_mut().take(IMAGES).zip(raw) {
        *page = *enabled == b'1';
    }
    visible[IMAGES] = true;
    visible[VIDEO] = true;
    visible[MONITOR] = true;
    if !visible.contains(&true) {
        visible[MUSIC] = true;
    }
    visible
}

pub fn migrate_current_v6_order(legacy: [usize; 6]) -> [usize; COUNT] {
    let mut order: Vec<_> = legacy.into_iter().filter(|page| *page < MONITOR).collect();
    let insert_at = order
        .iter()
        .position(|page| *page == VIDEO)
        .map(|index| index + 1)
        .or_else(|| {
            order
                .iter()
                .position(|page| *page == MUSIC)
                .map(|index| index + 1)
        })
        .unwrap_or(0);
    order.insert(insert_at.min(order.len()), MONITOR);
    finish_order(order)
}

pub fn migrate_current_v6_visibility(raw: &[u8]) -> [bool; COUNT] {
    let mut visible = [false; COUNT];
    for (page, enabled) in visible.iter_mut().take(MONITOR).zip(raw) {
        *page = *enabled == b'1';
    }
    visible[MONITOR] = true;
    if !visible.contains(&true) {
        visible[MUSIC] = true;
    }
    visible
}

fn finish_order(mut order: Vec<usize>) -> [usize; COUNT] {
    for page in ALL {
        if !order.contains(&page) {
            order.push(page);
        }
    }
    order.try_into().unwrap_or(ALL)
}
