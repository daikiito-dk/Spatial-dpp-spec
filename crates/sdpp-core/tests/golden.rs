//! Deterministic golden tests (AGENTS.md rule 2).
//! Reports, inspect output, and USDA export are compared byte-for-byte.
//! Regenerate goldens with `SDPP_UPDATE_GOLDEN=1 cargo test` — never hand-edit.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use sdpp_core::format::{self, Format};
use sdpp_core::validate::validate_bytes;
use sdpp_core::{inspect, usd};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn golden_dir(name: &str) -> PathBuf {
    repo_root().join("tests/golden").join(name)
}

fn sorted_files(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
        .flatten()
        .map(|entry| entry.path())
        .collect();
    files.sort();
    files
}

fn is_document_json(path: &Path) -> bool {
    path.extension() == Some(OsStr::new("json"))
        && !path.to_string_lossy().ends_with(".report.json")
}

fn update_golden() -> bool {
    std::env::var("SDPP_UPDATE_GOLDEN").is_ok()
}

fn check_or_write(path: &Path, rendered: &str) {
    if update_golden() {
        std::fs::write(path, rendered).unwrap_or_else(|e| panic!("write {}: {e}", path.display()));
        return;
    }
    let golden = std::fs::read_to_string(path).unwrap_or_else(|e| {
        panic!(
            "missing golden {} (run `SDPP_UPDATE_GOLDEN=1 cargo test` to generate): {e}",
            path.display()
        )
    });
    assert_eq!(rendered, golden, "golden mismatch: {}", path.display());
}

#[test]
fn valid_documents_match_golden_reports() {
    for path in sorted_files(&golden_dir("valid")) {
        if !is_document_json(&path) {
            continue;
        }
        let data = std::fs::read(&path).unwrap();
        let report = validate_bytes(&data, Some(Format::Json)).expect("fixture must parse as JSON");
        assert!(
            report.valid,
            "{} must be valid: {:?}",
            path.display(),
            report.errors
        );
        let rendered = serde_json::to_string_pretty(&report).unwrap();
        check_or_write(&path.with_extension("report.json"), &rendered);
    }
}

#[test]
fn invalid_documents_produce_expected_codes() {
    for path in sorted_files(&golden_dir("invalid")) {
        if path.extension() != Some(OsStr::new("json")) {
            continue;
        }
        let data = std::fs::read(&path).unwrap();
        let report = validate_bytes(&data, Some(Format::Json)).expect("fixture must parse as JSON");
        assert!(!report.valid, "{} must be invalid", path.display());
        let expected_raw = std::fs::read_to_string(path.with_extension("expected")).unwrap();
        let expected: serde_json::Value = serde_json::from_str(&expected_raw).unwrap();
        assert_eq!(expected["valid"], serde_json::Value::Bool(false));
        let mut codes: Vec<String> = report.errors.iter().map(|e| e.code.clone()).collect();
        codes.sort();
        codes.dedup();
        let expected_codes: Vec<String> = expected["codes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect();
        assert_eq!(
            codes,
            expected_codes,
            "unexpected codes for {}",
            path.display()
        );
    }
}

#[test]
fn binary_formats_agree_with_json_reports() {
    for path in sorted_files(&golden_dir("valid")) {
        if !is_document_json(&path) {
            continue;
        }
        let data = std::fs::read(&path).unwrap();
        let json_report = validate_bytes(&data, Some(Format::Json)).unwrap();
        let (_, doc) = format::parse_document(&data, Some(Format::Json)).unwrap();

        let packed = format::to_msgpack(&doc).unwrap();
        let msgpack_report = validate_bytes(&packed, Some(Format::MsgPack)).unwrap();
        assert!(
            msgpack_report.valid,
            "{} msgpack must validate",
            path.display()
        );
        assert_eq!(msgpack_report.errors, json_report.errors);
        assert_eq!(msgpack_report.warnings, json_report.warnings);

        let wire = format::to_protobuf(&doc);
        let proto_report = validate_bytes(&wire, Some(Format::Protobuf)).unwrap();
        assert!(
            proto_report.valid,
            "{} protobuf must validate",
            path.display()
        );
        assert_eq!(proto_report.errors, json_report.errors);
        assert_eq!(proto_report.warnings, json_report.warnings);
    }
}

#[test]
fn usd_export_is_deterministic_and_matches_golden() {
    for path in sorted_files(&golden_dir("valid")) {
        if !is_document_json(&path) {
            continue;
        }
        let data = std::fs::read(&path).unwrap();
        let (_, doc) = format::parse_document(&data, Some(Format::Json)).unwrap();
        let usda = usd::export_usda(&doc);
        assert_eq!(usda, usd::export_usda(&doc), "export must be deterministic");
        check_or_write(&path.with_extension("usda"), &usda);
    }
}

#[test]
fn inspect_output_is_deterministic_and_matches_golden() {
    for path in sorted_files(&golden_dir("valid")) {
        if !is_document_json(&path) {
            continue;
        }
        let data = std::fs::read(&path).unwrap();
        let (_, doc) = format::parse_document(&data, Some(Format::Json)).unwrap();
        let summary = inspect::summary(&doc);
        assert_eq!(summary, inspect::summary(&doc));
        check_or_write(&path.with_extension("inspect.txt"), &summary);
    }
}

#[test]
fn examples_are_identical_to_golden_fixtures() {
    let examples = repo_root().join("examples");
    for path in sorted_files(&examples) {
        let name = path.file_name().unwrap().to_str().unwrap().to_string();
        let golden = golden_dir("valid").join(&name);
        let example_bytes = std::fs::read(&path).unwrap();
        let golden_bytes = std::fs::read(&golden)
            .unwrap_or_else(|e| panic!("example {name} has no golden fixture: {e}"));
        assert_eq!(
            example_bytes, golden_bytes,
            "examples/{name} must stay in sync with tests/golden/valid/{name}"
        );
    }
}
