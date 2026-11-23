use anyhow::Result;
use hlp_core::types::{PlaybackPlan, UrlInfo};
use hlp_media::{
    decide, default_caps, extract_audio, prepare_playback, probe, Downloader, MediaServer, Tools,
};
use reqwest::{header, Client, StatusCode};
use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn registered_files_support_conditional_ranges_and_head() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let file = temp.path().join("movie.m4v");
    fs::write(&file, (0..200u8).collect::<Vec<_>>())?;
    let server = MediaServer::start().await?;
    let url = server.register(&file)?;
    let http = Client::builder().no_proxy().build()?;
    let head = http.head(&url).send().await?;
    assert_eq!(head.status(), StatusCode::OK);
    assert_eq!(head.headers()[header::CONTENT_LENGTH], "200");
    assert_eq!(head.headers()[header::CONTENT_TYPE], "video/mp4");
    let validator = head.headers()[header::LAST_MODIFIED].clone();
    assert!(head.bytes().await?.is_empty());
    let part = http
        .get(&url)
        .header(header::RANGE, "bytes=10-19")
        .header(header::IF_RANGE, validator)
        .send()
        .await?;
    assert_eq!(part.status(), StatusCode::PARTIAL_CONTENT);
    assert_eq!(part.headers()[header::CONTENT_RANGE], "bytes 10-19/200");
    assert_eq!(part.bytes().await?.as_ref(), (10..20u8).collect::<Vec<_>>());
    let all = http
        .get(&url)
        .header(header::RANGE, "bytes=10-19")
        .header(header::IF_RANGE, "\"old\"")
        .send()
        .await?;
    assert_eq!(all.status(), StatusCode::OK);
    assert_eq!(all.bytes().await?.len(), 200);
    server.unregister(&url);
    assert_eq!(http.get(url).send().await?.status(), StatusCode::NOT_FOUND);
    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn cancellation_kills_downloader_and_descendants_and_only_cleans_its_own_files() -> Result<()>
{
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir()?;
    let tool = root.path().join("fake yt-dlp");
    fs::write(
        &tool,
        r#"#!/bin/sh
for arg in "$@"; do
  case "$arg" in temp:*) staging=${arg#temp:};; home:*) out=${arg#home:};; esac
done
touch "$staging/fake.part" "$staging/fake.ytdl"
sleep 120 &
printf '%s\n' "$!" > "$out/descendant.pid"
wait
"#,
    )?;
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o755))?;
    let tools = Tools {
        ffmpeg: root.path().join("ffmpeg"),
        ffprobe: PathBuf::new(),
        deno: root.path().join("deno"),
        yt_dlp: tool,
    };
    let downloader = Downloader::new(tools);
    let info = UrlInfo {
        id: "id".into(),
        title: "title".into(),
        duration: Some(10.),
        extractor: "test".into(),
        thumbnail: None,
        is_live: false,
    };
    let cancel = CancellationToken::new();
    let token = cancel.clone();
    let dest = root.path().join("downloads");
    fs::create_dir(&dest)?;
    fs::write(dest.join("unrelated.part"), b"keep")?;
    let dir = dest.clone();
    let download = tokio::spawn(async move {
        downloader
            .download("https://example.invalid/video", &info, &dir, |_| {}, token)
            .await
    });
    let pid_file = dest.join("descendant.pid");
    let pid = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Some(pid) = fs::read_to_string(&pid_file)
                .ok()
                .and_then(|s| s.trim().parse::<i32>().ok())
            {
                break pid;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await?;
    cancel.cancel();
    let error = tokio::time::timeout(Duration::from_secs(5), download)
        .await??
        .unwrap_err();
    assert!(error.to_string().contains("Cancelled"));
    tokio::time::timeout(Duration::from_secs(5), async {
        while unsafe { libc::kill(pid, 0) } == 0 {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await?;
    assert_eq!(fs::read_dir(&dest)?.count(), 2);
    assert_eq!(fs::read(dest.join("unrelated.part"))?, b"keep");
    Ok(())
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name)
}

// Invoked by the media lane acceptance check after generating the spoken fixtures.
#[test]
#[ignore = "requires FFmpeg and generated fixtures"]
fn fixture_preparation_and_ten_minute_extraction() -> Result<()> {
    let tools = Tools::resolve();
    let dir = tempfile::tempdir()?;
    let caps = default_caps("wkwebview");
    for (name, expected) in [
        ("fixture_ja.mp4", PlaybackPlan::Direct),
        ("fixture_fr.mkv", PlaybackPlan::Remux),
        ("fixture_es.avi", PlaybackPlan::Transcode),
        ("fixture_de.webm", PlaybackPlan::Direct),
        ("fixture_zh.mov", PlaybackPlan::Direct),
    ] {
        let input = fixture(name);
        let info = probe(&tools, &input)?;
        let plan = decide(&info, &caps, 0);
        assert_eq!(plan, expected, "{name}");
        if plan != PlaybackPlan::Direct {
            let output = dir.path().join(format!("{name}.mp4"));
            let mut values = Vec::new();
            let result = prepare_playback(
                &tools,
                &input,
                &info,
                &caps,
                0,
                &plan,
                &output,
                |p| values.push(p),
                CancellationToken::new(),
            )?;
            let playable = probe(&tools, &result)?;
            assert_eq!(playable.video.unwrap().codec_name, "h264");
            assert!(matches!(
                playable.audio[0].codec_name.as_str(),
                "aac" | "mp3"
            ));
            assert_eq!(values.last(), Some(&1.));
        }
        println!("{name} {plan:?} {} ms", info.duration_ms);
    }
    let input = dir.path().join("ten-minutes.m4a");
    let status = std::process::Command::new(&tools.ffmpeg)
        .args(["-nostdin", "-hide_banner", "-v", "error", "-y", "-i"])
        .arg(fixture("fixture_long_en.mp4"))
        .args(["-vn", "-t", "600", "-c:a", "aac"])
        .arg(&input)
        .status()?;
    assert!(status.success());
    let info = probe(&tools, &input)?;
    let pcm = dir.path().join("audio.pcm");
    let mut count = 0;
    let samples = extract_audio(
        &tools,
        &input,
        &info.audio[0],
        true,
        &pcm,
        |frame, ms| {
            count += frame.len() as u64;
            assert_eq!(count / 16, ms);
            assert!(frame.len() <= 512);
            assert!(fs::metadata(&pcm)?.len() >= count * 2);
            Ok(())
        },
        CancellationToken::new(),
    )?;
    assert_eq!(samples, count);
    assert_eq!(fs::metadata(&pcm)?.len(), samples * 2);
    let expected = info.duration_ms * 16;
    println!("ten-minute PCM: {samples} samples, expected {expected}");
    assert!(samples.abs_diff(expected) <= expected / 1000);
    Ok(())
}

#[tokio::test]
#[ignore = "requires managed yt-dlp, Deno, FFmpeg and generated fixtures"]
async fn actual_downloader_on_a_loopback_fixture() -> Result<()> {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let tools = Tools::resolve();
    let server = MediaServer::start().await?;
    let source = fixture("fixture_ja.mp4");
    let url = server.register(&source)?;
    let downloader = Downloader::new(tools);
    let info = downloader.probe(&url, CancellationToken::new()).await?;
    assert!(!info.title.is_empty());
    let dir = tempfile::tempdir()?;
    let updates = AtomicUsize::new(0);
    let out = downloader
        .download(
            &url,
            &info,
            dir.path(),
            |p| {
                assert!(p.part > 0);
                updates.fetch_add(1, Ordering::Relaxed);
            },
            CancellationToken::new(),
        )
        .await?;
    assert_eq!(fs::read(out)?, fs::read(source)?);
    assert_eq!(fs::read_dir(dir.path())?.count(), 1);
    assert!(
        updates.load(Ordering::Relaxed) > 0,
        "Expected progress from real yt-dlp"
    );
    Ok(())
}
