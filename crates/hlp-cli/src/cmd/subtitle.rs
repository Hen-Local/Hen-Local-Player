#[derive(clap::Args)]
pub struct Args {
    pub input: String,
    #[arg(long, default_value = "auto")]
    pub source: String,
    #[arg(long)]
    pub to: Option<String>,
    #[arg(long, value_delimiter = ',', default_value = "srt,vtt,ass")]
    pub formats: Vec<String>,
    #[arg(long)]
    pub out_dir: Option<std::path::PathBuf>,
    #[arg(long)]
    pub no_cache: bool,
    #[arg(long)]
    pub json_progress: bool,
}
pub async fn run(args: Args) -> anyhow::Result<()> {
    todo!()
}
