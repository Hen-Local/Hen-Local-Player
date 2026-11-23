use anyhow::{Context, Result};
use hlp_core::types::ComponentState;
use indicatif::{ProgressBar, ProgressStyle};
use std::io::{IsTerminal, Write};
use tokio_util::sync::CancellationToken;

#[derive(clap::Args)]
pub struct Args {
    #[arg(long)]
    pub yes: bool,
    #[arg(long)]
    pub with_urls: bool,
}
pub async fn run(args: Args) -> Result<()> {
    let required = hlp_models::required_ids(args.with_urls);
    let missing: Vec<_> = hlp_models::status()
        .into_iter()
        .filter(|s| !s.ready && required.contains(&s.id))
        .collect();
    if missing.is_empty() {
        println!("All required components are ready.");
        return Ok(());
    }
    let total: u64 = missing.iter().map(|s| s.bytes_total).sum();
    for s in &missing {
        println!("{:<28} {:>12} bytes", s.id, s.bytes_total);
    }
    if !args.yes {
        if !std::io::stdin().is_terminal() {
            anyhow::bail!("Run setup --yes to download {total} bytes");
        }
        eprint!("Download {total} bytes? [y/N] ");
        std::io::stderr().flush()?;
        let mut answer = String::new();
        std::io::stdin().read_line(&mut answer)?;
        if !matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes") {
            return Ok(());
        }
    }
    let bar = ProgressBar::new(total);
    bar.set_style(ProgressStyle::with_template(
        "{msg} [{bar:28}] {bytes}/{total_bytes}",
    )?);
    let cancel = CancellationToken::new();
    let signal = cancel.clone();
    let interrupt = tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            signal.cancel();
        }
    });
    let ids: Vec<_> = missing.into_iter().map(|s| s.id).collect();
    let result = hlp_models::download(
        &ids,
        |s| {
            bar.set_length(s.bytes_total);
            bar.set_position(s.bytes_done);
            let state = match s.state {
                ComponentState::Verifying => "Verifying",
                ComponentState::Ready => "Ready",
                ComponentState::Failed(_) => "Failed",
                _ => "Downloading",
            };
            bar.set_message(format!("{state} {}", s.id));
        },
        cancel,
    )
    .await;
    interrupt.abort();
    bar.finish_and_clear();
    result.context(crate::ExitCode(2))?;
    println!("All required components are ready.");
    Ok(())
}
