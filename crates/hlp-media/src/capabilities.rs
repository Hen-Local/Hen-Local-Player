use hlp_core::types::{Caps, MediaInfo, PlaybackPlan};

const AVC: &str = r#"video/mp4; codecs="avc1.640028""#;
const HEVC: &str = r#"video/mp4; codecs="hvc1.1.6.L120.90""#;
const AV1: &str = r#"video/mp4; codecs="av01.0.08M.08""#;
const VP9: &str = r#"video/mp4; codecs="vp09.00.40.08""#;
const WEBM_AV1: &str = r#"video/webm; codecs="av01.0.08M.08""#;
const WEBM_VP9: &str = r#"video/webm; codecs="vp9""#;
const WEBM_VP8: &str = r#"video/webm; codecs="vp8""#;
const AAC: &str = r#"audio/mp4; codecs="mp4a.40.2""#;
const OPUS: &str = r#"audio/mp4; codecs="opus""#;
const FLAC: &str = r#"audio/mp4; codecs="flac""#;
const MP3: &str = "audio/mpeg";
const AC3: &str = r#"audio/mp4; codecs="ac-3""#;
const EAC3: &str = r#"audio/mp4; codecs="ec-3""#;
const WEBM_OPUS: &str = r#"audio/webm; codecs="opus""#;
const WEBM_VORBIS: &str = r#"audio/webm; codecs="vorbis""#;

pub fn default_caps(webview: &str) -> Caps {
    let mut caps: Caps = [
        AVC,
        AV1,
        VP9,
        WEBM_AV1,
        WEBM_VP9,
        WEBM_VP8,
        AAC,
        OPUS,
        FLAC,
        MP3,
        WEBM_OPUS,
        WEBM_VORBIS,
    ]
    .into_iter()
    .map(|key| (key.into(), "probably".into()))
    .collect();
    for key in [HEVC, AC3, EAC3] {
        caps.insert(
            key.into(),
            if webview == "wkwebview" {
                "probably"
            } else {
                ""
            }
            .into(),
        );
    }
    caps
}

fn can(caps: &Caps, key: &str) -> bool {
    matches!(
        caps.get(key).map(String::as_str),
        Some("maybe" | "probably")
    )
}

pub(crate) fn copy_audio(codec: &str, caps: &Caps) -> bool {
    can(
        caps,
        match codec {
            "aac" => AAC,
            "mp3" => MP3,
            "opus" => OPUS,
            "flac" => FLAC,
            _ => return false,
        },
    )
}
pub(crate) fn interlaced(field_order: &str) -> bool {
    matches!(field_order, "tt" | "bb" | "tb" | "bt" | "interlaced")
}

pub fn decide(info: &MediaInfo, caps: &Caps, selected_audio: usize) -> PlaybackPlan {
    use PlaybackPlan::*;
    let audio = info.audio.iter().find(|s| s.order == selected_audio);
    if !info.audio.is_empty() && audio.is_none() {
        return Transcode;
    }
    let is_default = info
        .audio
        .iter()
        .find(|s| s.default)
        .or(info.audio.first())
        .is_none_or(|s| s.order == selected_audio);
    let container = |name| info.container.split(',').any(|s| s == name);
    let mp4 = container("mov") || container("mp4") || container("m4a") || container("m4v");
    let audio_mp4 = audio.is_none_or(|audio| {
        copy_audio(&audio.codec_name, caps)
            || match audio.codec_name.as_str() {
                "ac3" => can(caps, AC3),
                "eac3" => can(caps, EAC3),
                _ => false,
            }
    });
    let Some(video) = &info.video else {
        let Some(audio) = audio else { return Direct };
        let direct = (mp4 && audio_mp4)
            || match audio.codec_name.as_str() {
                "mp3" => container("mp3") && can(caps, MP3),
                "flac" => container("flac") && can(caps, FLAC),
                "opus" => container("webm") && can(caps, WEBM_OPUS),
                "vorbis" => container("webm") && can(caps, WEBM_VORBIS),
                _ => false,
            };
        return if direct && is_default {
            Direct
        } else {
            Transcode
        };
    };
    if interlaced(&video.field_order) {
        return Transcode;
    }
    if is_default && container("webm") {
        let video_mime = match video.codec_name.as_str() {
            "vp9" => WEBM_VP9,
            "vp8" => WEBM_VP8,
            "av1" => WEBM_AV1,
            _ => "",
        };
        let audio_ok = audio.is_none_or(|audio| match audio.codec_name.as_str() {
            "opus" => can(caps, WEBM_OPUS),
            "vorbis" => can(caps, WEBM_VORBIS),
            _ => false,
        });
        if can(caps, video_mime) && audio_ok {
            return Direct;
        }
    }
    let video_mime = match video.codec_name.as_str() {
        "h264" => AVC,
        "hevc" => HEVC,
        "av1" => AV1,
        "vp9" => VP9,
        _ => return Transcode,
    };
    if !can(caps, video_mime) {
        return Transcode;
    }
    if is_default
        && mp4
        && audio_mp4
        && (video.codec_name != "hevc" || video.codec_tag_string == "hvc1")
    {
        Direct
    } else {
        Remux
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hlp_core::types::{AudioStream, VideoStream};
    #[test]
    fn hevc_tag_track_selection_and_decoder_capabilities() {
        let track = AudioStream {
            index: 1,
            order: 0,
            codec_name: "aac".into(),
            channels: 2,
            channel_layout: "stereo".into(),
            sample_rate: 48000,
            language_tag: None,
            default: true,
        };
        let mut info = MediaInfo {
            container: "mov,mp4".into(),
            duration_ms: 1000,
            has_subtitle_streams: false,
            video: Some(VideoStream {
                codec_name: "hevc".into(),
                codec_tag_string: "hev1".into(),
                profile: "Main".into(),
                pix_fmt: "yuv420p".into(),
                width: 1280,
                height: 720,
                field_order: "progressive".into(),
            }),
            audio: vec![
                track.clone(),
                AudioStream {
                    order: 1,
                    index: 2,
                    default: false,
                    ..track
                },
            ],
        };
        let mut caps = default_caps("wkwebview");
        assert_eq!(decide(&info, &caps, 0), PlaybackPlan::Remux);
        info.video.as_mut().unwrap().codec_tag_string = "hvc1".into();
        caps.insert(HEVC.into(), "maybe".into());
        assert_eq!(decide(&info, &caps, 0), PlaybackPlan::Direct);
        assert_eq!(decide(&info, &caps, 1), PlaybackPlan::Remux);
        assert_eq!(
            decide(&info, &default_caps("webview2"), 0),
            PlaybackPlan::Transcode
        );
        info.video.as_mut().unwrap().field_order = "tt".into();
        assert_eq!(decide(&info, &caps, 0), PlaybackPlan::Transcode);
    }
}
