use smartcal_core::Language;
use smartcal_i18n::{TextKey, bidi_visual_runs, tr, wrap_logical_lines};

#[test]
fn bidi_visual_runs_preserve_logical_persian_and_order_mixed_runs() {
    let runs = bidi_visual_runs("بررسی libinput و سیستم");
    // Visual left-to-right order of runs for RTL paragraph:
    // "و سیستم" (RTL) | " " | "libinput" (LTR) | " " | "بررسی" (RTL)
    assert!(runs.len() >= 3);
    assert!(runs.first().unwrap().text.contains("سیستم"));
    assert!(runs.last().unwrap().text.contains("بررسی"));
    assert!(runs.iter().any(|r| r.text == "libinput" && !r.is_rtl));
}

#[test]
fn logical_wrapping_preserves_top_to_bottom_sentence_order() {
    let lines = wrap_logical_lines("اسکن نمایشگرها، پروژکتور و کنترلرهای لمسی سیستم", 26);
    assert!(lines.len() >= 2);
    assert!(lines[0].starts_with("اسکن"));
    assert!(lines.last().unwrap().contains("سیستم"));
}

#[test]
fn english_and_persian_keys_resolve_cleanly() {
    assert_eq!(
        tr(Language::English, TextKey::TouchInaccurateError),
        "Touch point was not detected accurately. Please touch the target again."
    );
    assert_eq!(
        tr(Language::Persian, TextKey::QuickCalibrate),
        "کالیبره سریع (تمام‌صفحه)"
    );
}
