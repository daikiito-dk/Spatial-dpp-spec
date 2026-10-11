//! Round-trip tests: every pair of serializations must preserve semantics
//! (SPEC.md Section 7), including glTF embedding and protobuf error twins.

use std::path::PathBuf;

use prost::Message as _;
use serde_json::{json, Value};

use sdpp_core::format::{self, Format};
use sdpp_core::gltf;
use sdpp_core::validate::validate_bytes;
use sdpp_core::SpatialDppDocument;

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/golden/valid")
            .join(name),
    )
    .unwrap()
}

fn parse(data: &[u8]) -> SpatialDppDocument {
    format::parse_document(data, Some(Format::Json)).unwrap().1
}

fn canonical(doc: &SpatialDppDocument) -> String {
    format::to_json(doc).unwrap()
}

#[test]
fn canonicalization_is_stable_and_drops_transport_keys() {
    let data = fixture("ev_battery_pack.json");
    let once = canonical(&parse(&data));
    let twice = canonical(&parse(once.as_bytes()));
    assert_eq!(once, twice);
    // x-* keys and transport annotations never appear in canonical output
    assert!(!once.contains("x_note"));
    assert!(!once.contains("@context"));
    assert!(!once.contains("$schema"));
}

#[test]
fn msgpack_roundtrip_preserves_semantics() {
    let doc = parse(&fixture("ev_battery_pack.json"));
    let packed = format::to_msgpack(&doc).unwrap();
    let restored = format::parse_document(&packed, Some(Format::MsgPack))
        .unwrap()
        .1;
    assert_eq!(canonical(&doc), canonical(&restored));
}

#[test]
fn protobuf_roundtrip_preserves_semantics() {
    let doc = parse(&fixture("ev_battery_pack.json"));
    let wire = format::to_protobuf(&doc);
    let restored = format::parse_document(&wire, Some(Format::Protobuf))
        .unwrap()
        .1;
    assert_eq!(canonical(&doc), canonical(&restored));
}

#[test]
fn jsonld_roundtrip_preserves_semantics() {
    let doc = parse(&fixture("ev_battery_pack.json"));
    let ld = format::to_jsonld(&doc).unwrap();
    assert!(ld.contains("\"@context\":\"https://spatial-dpp.org/contexts/v0.1.0\""));
    assert!(ld.contains("\"@type\":\"SpatialDppDocument\""));
    let restored = format::parse_document(ld.as_bytes(), Some(Format::Jsonld))
        .unwrap()
        .1;
    assert_eq!(canonical(&doc), canonical(&restored));
}

#[test]
fn gltf_embed_then_extract_preserves_semantics() {
    let doc = parse(&fixture("ev_battery_pack.json"));
    let scene = json!({
        "asset": { "version": "2.0" },
        "nodes": [ { "name": "ev_pack" }, { "name": "pack_lid" } ]
    });
    let embedded = gltf::gltf_embed(&scene, &doc).unwrap();
    let used = embedded["extensionsUsed"].as_array().unwrap();
    assert!(used.iter().any(|v| v == "EXT_spatial_dpp"));
    assert_eq!(
        embedded["nodes"][0]["extensions"]["EXT_spatial_dpp"]["object"]["id"],
        json!("ev_pack")
    );
    let extracted = gltf::gltf_extract(&embedded).unwrap();
    assert_eq!(canonical(&doc), canonical(&extracted));
}

#[test]
fn gltf_extract_autofills_locator() {
    let mut doc = parse(&fixture("ev_battery_pack.json"));
    for object in &mut doc.objects {
        object.geometry.locator = None;
    }
    let scene = json!({
        "asset": { "version": "2.0" },
        "nodes": [ { "name": "ev_pack" }, { "name": "pack_lid" } ]
    });
    let embedded = gltf::gltf_embed(&scene, &doc).unwrap();
    let extracted = gltf::gltf_extract(&embedded).unwrap();
    assert_eq!(
        extracted.objects[0].geometry.locator,
        Some("#/nodes/0".to_string())
    );
    assert_eq!(
        extracted.objects[1].geometry.locator,
        Some("#/nodes/1".to_string())
    );
}

#[test]
fn gltf_embed_fails_without_matching_nodes() {
    let doc = parse(&fixture("ev_battery_pack.json"));
    let scene = json!({
        "asset": { "version": "2.0" },
        "nodes": [ { "name": "something_else" } ]
    });
    let err = gltf::gltf_embed(&scene, &doc).unwrap_err();
    assert!(err.to_string().contains("ev_pack"));
}

#[test]
fn protobuf_unknown_enum_string_reports_v01() {
    let wire = sdpp_core::proto::SpatialDppDocument {
        metadata: Some(sdpp_core::proto::DocumentMetadata {
            spec_version: "0.1.0".to_string(),
            document_id: "urn:test:x".to_string(),
            name: "X".to_string(),
            created: "2026-01-01T00:00:00Z".to_string(),
            author: None,
            language: None,
        }),
        objects: vec![sdpp_core::proto::SpatialObject {
            id: "o".to_string(),
            label: "L".to_string(),
            geometry: Some(sdpp_core::proto::GeometryRef {
                format: "fbx".to_string(), // unknown geometry format
                uri: "urn:test:mesh".to_string(),
                hash: None,
                locator: None,
                scale_hint_m: vec![],
            }),
            physical: None,
            compliance: None,
        }],
    };
    let bytes = wire.encode_to_vec();
    let report = validate_bytes(&bytes, Some(Format::Protobuf)).unwrap();
    assert!(!report.valid);
    let mut codes: Vec<String> = report.errors.iter().map(|e| e.code.clone()).collect();
    codes.sort();
    codes.dedup();
    assert_eq!(codes, vec!["V-01"]);
    assert!(report.errors.iter().any(|e| e.message.contains("fbx")));
}

#[test]
fn protobuf_missing_metadata_reports_v01() {
    let wire = sdpp_core::proto::SpatialDppDocument {
        metadata: None,
        objects: vec![],
    };
    let bytes = wire.encode_to_vec();
    let report = validate_bytes(&bytes, Some(Format::Protobuf)).unwrap();
    assert!(!report.valid);
    let codes: Vec<String> = report.errors.iter().map(|e| e.code.clone()).collect();
    assert_eq!(codes, vec!["V-01"]);
}

#[test]
fn schema_subset_catches_raw_json_violations() {
    // Feed a raw value that no typed parse could produce: geometry.format off-enum.
    let bad: Value = serde_json::from_str(
        r#"{"metadata":{"spec_version":"0.1.0","document_id":"urn:x:y","name":"N","created":"2026-01-01T00:00:00Z"},"objects":[]}"#,
    )
    .unwrap();
    let report = sdpp_core::validate::validate_value(&bad);
    // objects minItems twin fires semantically even when typed parse succeeds
    assert!(!report.valid);
    assert!(report
        .errors
        .iter()
        .any(|e| e.code == "V-01" && e.message.contains("at least 1 object")));
}
