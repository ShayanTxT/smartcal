use eframe::egui;
use smartcal_core::Language;
use smartcal_gui::fonts::configure_fonts;
use smartcal_gui::theme::bidi_job;

#[test]
fn persian_layout_job_shapes_rtl_right_to_left_in_harfrust() {
    let ctx = egui::Context::default();
    configure_fonts(&ctx);

    let out = ctx.run_ui(egui::RawInput::default(), |ui| {
        let job = bidi_job(
            Language::Persian,
            "شروع کالیبراسیون",
            16.0,
            egui::Color32::WHITE,
            None,
        );
        let galley = ui.ctx().fonts_mut(|f| f.layout_job(job));
        assert_eq!(galley.rows.len(), 1);
        let glyphs = &galley.rows[0].glyphs;
        assert!(
            glyphs.len() >= 10,
            "Expected shaped glyphs, got {}",
            glyphs.len()
        );
        assert!(galley.size().x > 40.0);

        let xs = glyphs
            .iter()
            .find(|g| g.chr == 'ش')
            .map(|g| g.pos.x)
            .expect("Expected 'ش' glyph");
        let xn = glyphs
            .iter()
            .find(|g| g.chr == 'ن')
            .map(|g| g.pos.x)
            .expect("Expected 'ن' glyph");
        assert!(
            xs > xn,
            "Expected first word 'شروع' on right (x={xs}) and end 'ن' on left (x={xn})"
        );
    });
    out.drop_without_applying_deltas();
}

#[test]
fn multiline_persian_layout_job_preserves_paragraph_rows() {
    let ctx = egui::Context::default();
    configure_fonts(&ctx);

    let out = ctx.run_ui(egui::RawInput::default(), |ui| {
        let job = bidi_job(
            Language::Persian,
            "اسکن نمایشگرها، پروژکتور و کنترلرهای لمسی سیستم",
            12.0,
            egui::Color32::GRAY,
            Some(28),
        );
        let galley = ui.ctx().fonts_mut(|f| f.layout_job(job));
        assert!(
            galley.rows.len() >= 2,
            "Expected wrapped rows, got {}",
            galley.rows.len()
        );
        // First row should be the first logical part ("اسکن ..."), not the tail word ("سیستم")
        let row0_chars: String = galley.rows[0].glyphs.iter().map(|g| g.chr).collect();
        assert!(
            row0_chars.contains('ا') && !row0_chars.contains("سیستم"),
            "Row 0 had unexpected text: {row0_chars}"
        );
    });
    out.drop_without_applying_deltas();
}
