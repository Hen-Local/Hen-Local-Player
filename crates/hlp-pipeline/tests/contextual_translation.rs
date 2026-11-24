//! Opt-in semantic smoke for the actual Hy-MT2 GGUF, using the same call as the player.
use hlp_pipeline::Translator;

#[tokio::test]
#[ignore = "requires the installed Hy-MT2 GGUF and llama-server"]
async fn identical_utterance_changes_sense_with_local_dialogue_context() -> anyhow::Result<()> {
    let mut translator = Translator::new();
    let source = "Look at the bat.";
    let baseball = translator.translate(
        Some("en"), "zh", source,
        Some("The pitcher is getting ready to throw the first ball.\nThe baseball players have brought their wooden equipment."),
    ).await?;
    let cave = translator.translate(
        Some("en"), "zh", source,
        Some("The sun has set outside the dark cave.\nThere are small flying mammals hanging from the ceiling."),
    ).await?;
    assert!(
        baseball.contains('棒') && !baseball.contains("蝙蝠"),
        "{baseball}"
    );
    assert!(cave.contains("蝙蝠"), "{cave}");
    for line in [baseball, cave] {
        assert!(
            line.lines().count() == 1 && line.chars().count() <= 20,
            "{line}"
        );
    }
    Ok(())
}
