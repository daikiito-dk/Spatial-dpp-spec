//! CLI contract tests (AGENTS.md rule 10): exit codes and captured output.

use std::path::PathBuf;

use sdpp_cli::run_with_output;

fn repo_path(rel: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
        .to_string_lossy()
        .into_owned()
}

fn args(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

fn stdout_string(out: &sdpp_cli::CliOutput) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn tmp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sdpp-cli-tests-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

#[test]
fn validate_accepts_a_valid_document() {
    let out = run_with_output(&args(&[
        "validate",
        &repo_path("tests/golden/valid/ev_battery_pack.json"),
    ]));
    assert_eq!(out.code, 0, "stderr: {}", out.stderr);
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["valid"], serde_json::Value::Bool(true));
    assert_eq!(report["object_count"], serde_json::Value::from(2));
}

#[test]
fn validate_rejects_an_invalid_document_with_code_1() {
    let out = run_with_output(&args(&[
        "validate",
        &repo_path("tests/golden/invalid/negative_mass.json"),
    ]));
    assert_eq!(out.code, 1);
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["valid"], serde_json::Value::Bool(false));
    assert!(report["errors"][0]["code"].as_str().unwrap() == "V-02");
}

#[test]
fn validate_flags_warnings_for_valid_documents() {
    let out = run_with_output(&args(&[
        "validate",
        &repo_path("tests/golden/valid/ev_battery_pack.json"),
    ]));
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let warning_codes: Vec<&str> = report["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| w["code"].as_str().unwrap())
        .collect();
    assert!(warning_codes.contains(&"W-03"));
}

#[test]
fn validate_reports_missing_file_with_code_2() {
    let out = run_with_output(&args(&["validate", "/definitely/not/a/file.json"]));
    assert_eq!(out.code, 2);
    assert!(out.stderr.contains("cannot read"));
}

#[test]
fn validate_rejects_unknown_format_flag() {
    let out = run_with_output(&args(&[
        "validate",
        &repo_path("tests/golden/valid/ev_battery_pack.json"),
        "--format",
        "yaml",
    ]));
    assert_eq!(out.code, 2);
    assert!(out.stderr.contains("unknown --format"));
}

#[test]
fn validate_supports_msgpack_input() {
    let source = repo_path("tests/golden/valid/ev_battery_pack.json");
    let packed = tmp("validate-msgpack.bin");
    let convert = run_with_output(&args(&[
        "convert",
        &source,
        "--to",
        "msgpack",
        "--out",
        packed.to_str().unwrap(),
    ]));
    assert_eq!(convert.code, 0, "stderr: {}", convert.stderr);
    let out = run_with_output(&args(&["validate", packed.to_str().unwrap()]));
    assert_eq!(out.code, 0, "stderr: {}", out.stderr);
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["valid"], serde_json::Value::Bool(true));
}

#[test]
fn inspect_prints_human_readable_summary() {
    let out = run_with_output(&args(&[
        "inspect",
        &repo_path("tests/golden/valid/ev_battery_pack.json"),
    ]));
    assert_eq!(out.code, 0, "stderr: {}", out.stderr);
    let text = stdout_string(&out);
    assert!(text.contains("Document:   Generic EV Battery Pack Module A"));
    assert!(text.contains("[ev_pack] EV Battery Pack"));
    assert!(text.contains("Mass:        12.4 kg"));
    assert!(text.contains("Passport:    urn:dpp:eu:ev-pack-2026"));
}

#[test]
fn export_produces_usda() {
    let out = run_with_output(&args(&[
        "export",
        &repo_path("tests/golden/valid/ev_battery_pack.json"),
        "--target=usd",
    ]));
    assert_eq!(out.code, 0, "stderr: {}", out.stderr);
    let text = stdout_string(&out);
    assert!(text.starts_with("#usda 1.0"));
    assert!(text.contains("double physics:mass = 12.4"));
    assert!(text.contains("def PhysicsRevoluteJoint \"joint_hinge_lid\""));
    assert!(text.contains("custom dictionary sdpp:compliance"));
}

#[test]
fn export_rejects_unsupported_targets() {
    let out = run_with_output(&args(&[
        "export",
        &repo_path("tests/golden/valid/ev_battery_pack.json"),
        "--target=gltf",
    ]));
    assert_eq!(out.code, 2);
    assert!(out.stderr.contains("unsupported --target"));
}

#[test]
fn convert_msgpack_and_back_roundtrips_canonical_json() {
    let source = repo_path("tests/golden/valid/ev_battery_pack.json");
    let packed = tmp("roundtrip.bin");
    let restored = tmp("roundtrip.json");

    let to = run_with_output(&args(&[
        "convert",
        &source,
        "--to",
        "msgpack",
        "--out",
        packed.to_str().unwrap(),
    ]));
    assert_eq!(to.code, 0, "stderr: {}", to.stderr);
    assert!(to.stdout.is_empty()); // --out suppresses stdout

    let back = run_with_output(&args(&[
        "convert",
        packed.to_str().unwrap(),
        "--to",
        "json",
        "--out",
        restored.to_str().unwrap(),
    ]));
    assert_eq!(back.code, 0, "stderr: {}", back.stderr);

    let original = std::fs::read(&source).unwrap();
    let converted = std::fs::read(&restored).unwrap();
    let original_canonical = canonical_of(&original);
    let converted_canonical = canonical_of(&converted);
    assert_eq!(original_canonical, converted_canonical);
}

fn canonical_of(data: &[u8]) -> String {
    let doc = sdpp_core::format::parse_document(data, Some(sdpp_core::Format::Json))
        .unwrap()
        .1;
    sdpp_core::format::to_json(&doc).unwrap()
}

#[test]
fn convert_requires_to_flag() {
    let out = run_with_output(&args(&[
        "convert",
        &repo_path("tests/golden/valid/ev_battery_pack.json"),
    ]));
    assert_eq!(out.code, 2);
    assert!(out.stderr.contains("--to"));
}

#[test]
fn gltf_embed_and_extract_roundtrip() {
    let doc = repo_path("tests/golden/valid/ev_battery_pack.json");
    let scene_path = tmp("scene.gltf");
    std::fs::write(
        &scene_path,
        serde_json::json!({ "asset": { "version": "2.0" }, "nodes": [ { "name": "ev_pack" }, { "name": "pack_lid" } ] }).to_string(),
    )
    .unwrap();

    let embedded_path = tmp("embedded.gltf");
    let embed = run_with_output(&args(&[
        "gltf-embed",
        &doc,
        scene_path.to_str().unwrap(),
        "--out",
        embedded_path.to_str().unwrap(),
    ]));
    assert_eq!(embed.code, 0, "stderr: {}", embed.stderr);
    let embedded_text = std::fs::read_to_string(&embedded_path).unwrap();
    assert!(embedded_text.contains("EXT_spatial_dpp"));

    let extract = run_with_output(&args(&["gltf-extract", embedded_path.to_str().unwrap()]));
    assert_eq!(extract.code, 0, "stderr: {}", extract.stderr);
    let extracted = stdout_string(&extract);
    let original = std::fs::read(doc).unwrap();
    assert_eq!(
        canonical_of(original.as_slice()),
        canonical_of(extracted.as_bytes())
    );
}

#[test]
fn gltf_embed_fails_on_name_mismatch() {
    let doc = repo_path("tests/golden/valid/ev_battery_pack.json");
    let scene_path = tmp("scene-mismatch.gltf");
    std::fs::write(
        &scene_path,
        serde_json::json!({ "asset": { "version": "2.0" }, "nodes": [ { "name": "other" } ] })
            .to_string(),
    )
    .unwrap();
    let out = run_with_output(&args(&["gltf-embed", &doc, scene_path.to_str().unwrap()]));
    assert_eq!(out.code, 2);
    assert!(out.stderr.contains("ev_pack"));
}

#[test]
fn help_and_version_exit_zero() {
    let help = run_with_output(&args(&["--help"]));
    assert_eq!(help.code, 0);
    assert!(stdout_string(&help).contains("COMMANDS"));
    let version = run_with_output(&args(&["--version"]));
    assert_eq!(version.code, 0);
    assert!(stdout_string(&version).contains("spatial-dpp 0.1.0"));
}

#[test]
fn unknown_command_exits_two() {
    let out = run_with_output(&args(&["transmogrify"]));
    assert_eq!(out.code, 2);
    assert!(out.stderr.contains("unknown command"));
}
