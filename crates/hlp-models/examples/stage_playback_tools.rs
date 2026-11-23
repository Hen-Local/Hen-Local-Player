// Stage the pinned playback tools and voice detector from the component installer.
use anyhow::{Context, Result};
use std::{fs, path::PathBuf};
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> Result<()> {
    let mut args = std::env::args_os().skip(1);
    let destination = PathBuf::from(args.next().context("Expected staging directory")?);
    let triple = args.next().context("Expected target triple")?;
    let triple = triple.to_str().context("Invalid target triple")?;
    let license = args.next().map(PathBuf::from);
    fs::create_dir_all(&destination)?;
    hlp_models::download(
        &["ffmpeg".into(), "vad-silero".into()],
        |_| {},
        CancellationToken::new(),
    )
    .await?;
    fs::copy(hlp_models::vad_path(), destination.join("silero_vad.onnx"))
        .context("Stage the verified speech detector")?;
    for name in ["ffmpeg", "ffprobe"] {
        let binary = hlp_models::tool_path(name);
        let filename = format!("{name}-{triple}{}", std::env::consts::EXE_SUFFIX);
        fs::copy(&binary, destination.join(filename)).with_context(|| format!("Stage {name}"))?;
    }
    let source =
        license.unwrap_or_else(|| hlp_models::tool_path("ffmpeg").with_file_name("LICENSE.txt"));
    fs::copy(source, destination.join("FFmpeg-LICENSE.txt")).context("Stage FFmpeg license")?;
    Ok(())
}
