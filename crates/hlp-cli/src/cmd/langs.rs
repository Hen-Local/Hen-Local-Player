#[derive(clap::Args)]
pub struct Args {}
pub async fn run(_: Args) -> anyhow::Result<()> {
    println!("Code\tLanguage\tASR\tTranslation target");
    for lang in hlp_core::lang::languages() {
        println!(
            "{}\t{}\t{}\t{}",
            lang.code,
            lang.english_name,
            if lang.qwen3_asr_name.is_some() {
                "yes"
            } else {
                "–"
            },
            if lang.hy_mt2_target { "yes" } else { "–" }
        );
    }
    Ok(())
}
