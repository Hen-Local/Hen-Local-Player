#[derive(clap::Args)]
pub struct Args {
    pub input: std::path::PathBuf,
    #[arg(long,default_value="wkwebview",value_parser=["wkwebview","webview2"])]
    pub webview: String,
}

pub async fn run(args: Args) -> anyhow::Result<()> {
    let tools = hlp_media::Tools::resolve();
    let info = hlp_media::probe(&tools, &args.input).map_err(|e| e.context(crate::ExitCode(3)))?;
    let track = info.audio.iter().find(|s| s.default).map_or(0, |s| s.order);
    let plan = hlp_media::decide(&info, &hlp_media::default_caps(&args.webview), track);
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "mediaInfo": info, "audioTrackOrder": track, "plan": plan
        }))?
    );
    Ok(())
}
