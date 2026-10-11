//! TypeScript/WASM bindings for sdpp-core.
//!
//! Build with:
//! ```text
//! wasm-pack build crates/sdpp-wasm --out-dir pkg --target web
//! ```
//! `pkg/` is generated output (gitignored); the hand-written TypeScript type
//! surface lives in `crates/sdpp-wasm/typescript/index.d.ts`.

use wasm_bindgen::prelude::*;

use sdpp_core::format::{self, Format};
use sdpp_core::validate::validate_bytes;

#[wasm_bindgen]
pub fn version() -> String {
    format!("spatial-dpp {}", sdpp_core::SPEC_VERSION)
}

/// Validate a JSON (or JSON-LD) document; always returns a report JSON
/// string, embedding parse failures as `V-01` issues.
#[wasm_bindgen]
pub fn validate_json(src: &str) -> String {
    match validate_bytes(src.as_bytes(), Some(Format::Json)) {
        Ok(report) => serde_json::to_string(&report)
            .unwrap_or_else(|_| error_report("report serialization failed")),
        Err(e) => error_report(&e.to_string()),
    }
}

/// Human-readable summary of a JSON document (see `spatial-dpp inspect`).
#[wasm_bindgen]
pub fn inspect_json(src: &str) -> Result<String, JsError> {
    let (_, doc) = format::parse_document(src.as_bytes(), Some(Format::Json))
        .map_err(|e| JsError::new(&e.to_string()))?;
    Ok(sdpp_core::inspect::summary(&doc))
}

/// JSON → canonical JSON with JSON-LD `@context`/`@type`.
#[wasm_bindgen]
pub fn to_jsonld(src: &str) -> Result<String, JsError> {
    let (_, doc) = parse(src)?;
    format::to_jsonld(&doc).map_err(|e| JsError::new(&e.to_string()))
}

/// JSON → MessagePack bytes.
#[wasm_bindgen]
pub fn to_msgpack(src: &str) -> Result<Vec<u8>, JsError> {
    let (_, doc) = parse(src)?;
    format::to_msgpack(&doc).map_err(|e| JsError::new(&e.to_string()))
}

/// MessagePack bytes → canonical JSON.
#[wasm_bindgen]
pub fn from_msgpack(bytes: &[u8]) -> Result<String, JsError> {
    let (_, doc) = format::parse_document(bytes, Some(Format::MsgPack))
        .map_err(|e| JsError::new(&e.to_string()))?;
    format::to_json(&doc).map_err(|e| JsError::new(&e.to_string()))
}

/// JSON → Protobuf bytes.
#[wasm_bindgen]
pub fn to_protobuf(src: &str) -> Result<Vec<u8>, JsError> {
    let (_, doc) = parse(src)?;
    Ok(format::to_protobuf(&doc))
}

/// Protobuf bytes → canonical JSON.
#[wasm_bindgen]
pub fn from_protobuf(bytes: &[u8]) -> Result<String, JsError> {
    let (_, doc) = format::parse_document(bytes, Some(Format::Protobuf))
        .map_err(|e| JsError::new(&e.to_string()))?;
    format::to_json(&doc).map_err(|e| JsError::new(&e.to_string()))
}

fn parse(src: &str) -> Result<(Format, sdpp_core::model::SpatialDppDocument), JsError> {
    format::parse_document(src.as_bytes(), Some(Format::Json))
        .map_err(|e| JsError::new(&e.to_string()))
}

fn error_report(message: &str) -> String {
    serde_json::json!({
        "valid": false,
        "errors": [
            { "code": "V-01", "path": "", "message": message }
        ],
        "warnings": [],
        "object_count": 0
    })
    .to_string()
}
