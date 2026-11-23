#[cfg(target_os = "macos")]
#[test]
#[ignore = "requires the generated speech fixtures and installed subtitle models"]
fn real_fixture_matrix() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    assert!(std::process::Command::new("bash")
        .arg("scripts/e2e.sh")
        .current_dir(root)
        .status()
        .unwrap()
        .success());
}
