use unicode_bidi::{BidiInfo, Level};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BidiRun {
    pub text: String,
    pub is_rtl: bool,
}

pub fn contains_rtl_script(input: &str) -> bool {
    input
        .chars()
        .any(|c| matches!(c, '\u{0600}'..='\u{06FF}' | '\u{FB50}'..='\u{FEFF}'))
}

/// Splits a single line of logical UTF-8 text into visual left-to-right runs without
/// altering logical character order inside each run (so HarfBuzz/harfrust shapes each
/// RTL or LTR run natively without double-reversing).
pub fn bidi_visual_runs(line: &str) -> Vec<BidiRun> {
    if !contains_rtl_script(line) {
        return vec![BidiRun {
            text: line.to_string(),
            is_rtl: false,
        }];
    }

    let bidi = BidiInfo::new(line, Some(Level::rtl()));
    let mut out = Vec::new();
    for para in &bidi.paragraphs {
        let (levels, runs) = bidi.visual_runs(para, para.range.clone());
        for run in runs {
            let level = levels[run.start];
            let slice = &line[run];
            if !slice.is_empty() {
                out.push(BidiRun {
                    text: slice.to_string(),
                    is_rtl: level.is_rtl(),
                });
            }
        }
    }
    out
}

/// Wraps logical text into lines of at most `max_chars` characters on word boundaries
/// before BiDi visual reordering so multi-line RTL paragraphs always flow top-to-bottom.
pub fn wrap_logical_lines(input: &str, max_chars: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for raw_line in input.split('\n') {
        if raw_line.chars().count() <= max_chars || max_chars < 8 {
            lines.push(raw_line.to_string());
            continue;
        }
        let mut current = String::new();
        let mut cur_len = 0usize;
        for word in raw_line.split_whitespace() {
            let wlen = word.chars().count();
            if cur_len > 0 && cur_len + 1 + wlen > max_chars {
                lines.push(current);
                current = word.to_string();
                cur_len = wlen;
            } else {
                if cur_len > 0 {
                    current.push(' ');
                    cur_len += 1;
                }
                current.push_str(word);
                cur_len += wlen;
            }
        }
        if !current.is_empty() {
            lines.push(current);
        }
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}
