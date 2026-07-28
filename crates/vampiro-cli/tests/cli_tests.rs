#[test]
fn cli_snapshots() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/cmd");
    trycmd::TestCases::new()
        .case(path.join("help.toml"))
        .case(path.join("version.toml"))
        .case(path.join("check-help.toml"))
        .case(path.join("prove-help.toml"));
}
