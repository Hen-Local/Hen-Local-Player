#[derive(clap::Args)]
pub struct Args {}

pub async fn run(_: Args) -> anyhow::Result<()> {
    println!("{:<30} {:<8} {:>12}", "Component", "State", "Bytes");
    for item in hlp_models::status() {
        println!(
            "{:<30} {:<8} {:>12}",
            item.id,
            if item.ready { "Ready" } else { "Missing" },
            item.bytes_total
        );
    }
    Ok(())
}
