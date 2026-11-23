mod cmd;
use clap::{Parser, Subcommand};
#[derive(Parser)]
#[command(version, about = "Local media subtitles")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand)]
enum Command {
    Setup(cmd::setup::Args),
    Status(cmd::status::Args),
    Probe(cmd::probe::Args),
    Subtitle(cmd::subtitle::Args),
    Langs(cmd::langs::Args),
}
#[derive(Debug)]
pub struct ExitCode(pub i32);
impl std::fmt::Display for ExitCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "exit {}", self.0)
    }
}
impl std::error::Error for ExitCode {}
#[tokio::main]
async fn main() {
    let result = match Cli::parse().command {
        Command::Setup(args) => cmd::setup::run(args).await,
        Command::Status(args) => cmd::status::run(args).await,
        Command::Probe(args) => cmd::probe::run(args).await,
        Command::Subtitle(args) => cmd::subtitle::run(args).await,
        Command::Langs(args) => cmd::langs::run(args).await,
    };
    if let Err(error) = result {
        eprintln!("{error:#}");
        std::process::exit(error.downcast_ref::<ExitCode>().map_or(1, |e| e.0));
    }
}
