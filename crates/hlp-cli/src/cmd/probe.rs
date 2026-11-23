#[derive(clap::Args)]
pub struct Args {
    pub input: std::path::PathBuf,
    #[arg(long,default_value="wkwebview",value_parser=["wkwebview","webview2"])]
    pub webview: String,
}
pub async fn run(args: Args) -> anyhow::Result<()> {
    todo!()
}
