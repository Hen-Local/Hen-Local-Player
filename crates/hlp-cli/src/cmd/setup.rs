#[derive(clap::Args)]
pub struct Args {
    #[arg(long)]
    pub yes: bool,
    #[arg(long)]
    pub with_urls: bool,
}
pub async fn run(args: Args) -> anyhow::Result<()> {
    todo!()
}
