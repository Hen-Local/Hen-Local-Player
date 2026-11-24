use crate::{cues::generated_line, types::*};
use std::fmt::Write;

fn timestamp(ms: Ms, format: &SubtitleFormat) -> String {
    if matches!(format, SubtitleFormat::Ass) {
        let cs = ms / 10 + u64::from(ms % 10 >= 5);
        return format!(
            "{}:{:02}:{:02}.{:02}",
            cs / 360000,
            cs / 6000 % 60,
            cs / 100 % 60,
            cs % 100
        );
    }
    let sep = if matches!(format, SubtitleFormat::Vtt) {
        '.'
    } else {
        ','
    };
    format!(
        "{:02}:{:02}:{:02}{sep}{:03}",
        ms / 3600000,
        ms / 60000 % 60,
        ms / 1000 % 60,
        ms % 1000
    )
}
fn ass_header() -> String {
    let font = if cfg!(windows) {
        "Microsoft YaHei"
    } else {
        "PingFang SC"
    };
    format!("[Script Info]\nScriptType: v4.00+\nPlayResX: 1920\nPlayResY: 1080\nScaledBorderAndShadow: yes\nWrapStyle: 2\n\n[V4+ Styles]\nFormat: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding\nStyle: Trans,{font},64,&H00FFFFFF,&H00FFFFFF,&H00000000,&H00000000,0,0,0,0,100,100,0,0,1,3,0,2,60,60,60,1\nStyle: Orig,{font},46,&H00E0E0E0,&H00FFFFFF,&H00000000,&H00000000,0,0,0,0,100,100,0,0,1,2,0,2,60,60,60,1\n\n[Events]\nFormat: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\n")
}
pub fn render(cues: &[Cue], format: &SubtitleFormat, variant: &SubtitleVariant) -> String {
    let mut out = match format {
        SubtitleFormat::Srt => String::new(),
        SubtitleFormat::Vtt => "WEBVTT\n\n".to_owned(),
        SubtitleFormat::Ass => ass_header(),
    };
    let mut index = 0;
    for cue in cues {
        let original = generated_line(&cue.original);
        let original = (!original.is_empty()).then_some(("Orig", original));
        let translation = cue
            .translation
            .as_deref()
            .map(generated_line)
            .filter(|s| !s.is_empty())
            .map(|s| ("Trans", s));
        let lines: Vec<_> = match variant {
            SubtitleVariant::Original => original.into_iter().collect(),
            SubtitleVariant::Translation => translation.into_iter().collect(),
            SubtitleVariant::Bilingual(BilingualOrder::OriginalFirst) => {
                original.into_iter().chain(translation).collect()
            }
            SubtitleVariant::Bilingual(BilingualOrder::TranslationFirst) => {
                translation.into_iter().chain(original).collect()
            }
        };
        if lines.is_empty() {
            continue;
        }
        index += 1;
        let start = timestamp(cue.start_ms, format);
        let end = timestamp(cue.end_ms, format);
        if matches!(format, SubtitleFormat::Ass) {
            let style = lines[0].0;
            let text = lines
                .into_iter()
                .map(|(style, text)| format!("{{\\r{style}}}{text}"))
                .collect::<Vec<_>>()
                .join("\\N");
            writeln!(out, "Dialogue: 0,{start},{end},{style},,0,0,0,,{text}").unwrap();
        } else {
            if matches!(format, SubtitleFormat::Srt) {
                writeln!(out, "{index}").unwrap();
            }
            writeln!(out, "{start} --> {end}").unwrap();
            for (_, mut text) in lines {
                if matches!(format, SubtitleFormat::Vtt) {
                    text = text
                        .replace('&', "&amp;")
                        .replace('<', "&lt;")
                        .replace('>', "&gt;");
                }
                writeln!(out, "{text}").unwrap();
            }
            out.push('\n');
        }
    }
    out
}
pub fn filename(
    stem: &str,
    source: &str,
    target: &str,
    format: &SubtitleFormat,
    variant: &SubtitleVariant,
) -> String {
    let lang = match variant {
        SubtitleVariant::Original => source.to_owned(),
        SubtitleVariant::Translation => target.to_owned(),
        SubtitleVariant::Bilingual(_) => format!("{source}-{target}"),
    };
    let ext = match format {
        SubtitleFormat::Srt => "srt",
        SubtitleFormat::Vtt => "vtt",
        SubtitleFormat::Ass => "ass",
    };
    format!("{stem}.{lang}.{ext}")
}
