use eframe::egui;
use std::path::Path;
use std::sync::Arc;

/// Loads a Persian/Arabic-capable font cleanly without scanning the entire font directory unless needed.
pub fn configure_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    if let Some((font_name, bytes)) = find_persian_capable_font() {
        fonts.font_data.insert(
            font_name.clone(),
            Arc::new(egui::FontData::from_owned(bytes)),
        );
        // Put Persian+Latin capable font at index 0 so spaces (' ') and Persian letters share
        // the same font face and harfrust shapes complete RTL phrases cleanly.
        if let Some(fam) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
            fam.insert(0, font_name.clone());
        }
        if let Some(fam) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
            fam.push(font_name);
        }
    }

    ctx.set_fonts(fonts);
}

fn find_persian_capable_font() -> Option<(String, Vec<u8>)> {
    // Fast direct paths first (avoids scanning hundreds of system fonts at startup).
    let fast_paths = [
        // Linux Persian / Arabic / DejaVu standard paths
        (
            "Vazirmatn",
            "/usr/share/fonts/truetype/vazirmatn/Vazirmatn-Regular.ttf",
        ),
        (
            "Vazirmatn",
            "/usr/share/fonts/opentype/vazirmatn/Vazirmatn-Regular.otf",
        ),
        (
            "Noto Sans Arabic",
            "/usr/share/fonts/truetype/noto/NotoSansArabic-Regular.ttf",
        ),
        (
            "Noto Naskh Arabic",
            "/usr/share/fonts/truetype/noto/NotoNaskhArabic-Regular.ttf",
        ),
        (
            "DejaVu Sans",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        ),
        ("DejaVu Sans", "/usr/share/fonts/TTF/DejaVuSans.ttf"),
        (
            "DejaVu Sans",
            "/usr/share/fonts/dejavu-sans-fonts/DejaVuSans.ttf",
        ),
        (
            "FreeSans",
            "/usr/share/fonts/truetype/freefont/FreeSans.ttf",
        ),
        // Windows standard paths (for preview/testing)
        ("Tahoma", r"C:\Windows\Fonts\tahoma.ttf"),
        ("Segoe UI", r"C:\Windows\Fonts\segoeui.ttf"),
        ("Arial", r"C:\Windows\Fonts\arial.ttf"),
    ];

    for (name, path_str) in fast_paths {
        let p = Path::new(path_str);
        if p.exists()
            && let Ok(bytes) = std::fs::read(p)
        {
            return Some((name.to_string(), bytes));
        }
    }

    // Fallback to fontdb lookup on non-standard Linux distributions
    let mut db = fontdb::Database::new();
    db.load_system_fonts();

    let preferred_families = [
        "Vazirmatn",
        "Vazir",
        "Sahel",
        "Noto Sans Arabic",
        "Noto Naskh Arabic",
        "DejaVu Sans",
        "Tahoma",
        "Segoe UI",
        "Arial",
        "FreeSans",
    ];

    for family in preferred_families {
        let query = fontdb::Query {
            families: &[fontdb::Family::Name(family)],
            weight: fontdb::Weight::NORMAL,
            stretch: fontdb::Stretch::Normal,
            style: fontdb::Style::Normal,
        };
        if let Some(id) = db.query(&query) {
            let mut found_bytes = None;
            db.with_face_data(id, |data, _| {
                found_bytes = Some(data.to_vec());
            });
            if let Some(bytes) = found_bytes {
                return Some((family.to_string(), bytes));
            }
        }
    }

    None
}
