//! Format detection, decoding, and encoding for the four serializations
//! (SPEC.md Section 7): canonical JSON, JSON-LD, MessagePack, Protobuf.

use serde_json::Value;

use crate::model::SpatialDppDocument;
use crate::proto;
use crate::validate::ValidationIssue;

/// Input/output formats. `Jsonld` shares the JSON wire form; serialization adds
/// `@context`/`@type` and parsing accepts and ignores them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Json,
    Jsonld,
    MsgPack,
    Protobuf,
}

impl Format {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "json" => Some(Format::Json),
            "jsonld" | "json-ld" => Some(Format::Jsonld),
            "msgpack" | "messagepack" => Some(Format::MsgPack),
            "proto" | "protobuf" => Some(Format::Protobuf),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Format::Json => "json",
            Format::Jsonld => "jsonld",
            Format::MsgPack => "msgpack",
            Format::Protobuf => "protobuf",
        }
    }
}

impl std::fmt::Display for Format {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error(
        "could not detect the input format (expected JSON, JSON-LD, MessagePack, or Protobuf)"
    )]
    UnknownFormat,
    #[error("JSON parse error: {0}")]
    Json(String),
    #[error("MessagePack decode error: {0}")]
    MsgPack(String),
    #[error("Protobuf decode error: {0}")]
    Protobuf(String),
    /// Protobuf documents that violate the structural model (unknown enum
    /// strings, missing required messages). Carries the `V-01` issues.
    #[error("document does not conform to the structural model")]
    InvalidModel(Vec<ValidationIssue>),
    #[error("serialization error: {0}")]
    Serialize(String),
}

/// Detection order per SPEC.md 7.5: JSON (first non-whitespace byte `{`),
/// MessagePack (probe decode must yield a top-level map), then Protobuf.
pub fn detect_format(data: &[u8]) -> Option<Format> {
    let starts_json = data
        .iter()
        .find(|b| !b.is_ascii_whitespace())
        .is_some_and(|b| *b == b'{');
    if starts_json {
        return Some(Format::Json);
    }
    if let Ok(probe) = rmp_serde::from_slice::<Value>(data) {
        if probe.is_object() {
            return Some(Format::MsgPack);
        }
    }
    if proto::decode_wire(data).is_ok() {
        return Some(Format::Protobuf);
    }
    None
}

/// Decode any supported format into the canonical JSON value form.
/// Returns the format actually used (normalized: JSON-LD decodes as JSON).
pub fn decode_to_value(data: &[u8], format: Option<Format>) -> Result<(Format, Value), ParseError> {
    let fmt = match format {
        Some(Format::Jsonld) => Format::Json,
        Some(f) => f,
        None => detect_format(data).ok_or(ParseError::UnknownFormat)?,
    };
    match fmt {
        Format::Json => {
            let value = serde_json::from_slice::<Value>(data)
                .map_err(|e| ParseError::Json(e.to_string()))?;
            Ok((Format::Json, value))
        }
        Format::MsgPack => {
            let value = rmp_serde::from_slice::<Value>(data)
                .map_err(|e| ParseError::MsgPack(e.to_string()))?;
            if !value.is_object() {
                return Err(ParseError::MsgPack(
                    "top-level MessagePack value is not a map".into(),
                ));
            }
            Ok((Format::MsgPack, value))
        }
        Format::Protobuf => {
            let wire = proto::decode_wire(data).map_err(|e| ParseError::Protobuf(e.to_string()))?;
            match proto::proto_to_model(&wire) {
                Ok(model) => {
                    let value = serde_json::to_value(&model)
                        .map_err(|e| ParseError::Serialize(e.to_string()))?;
                    Ok((Format::Protobuf, value))
                }
                Err(issues) => Err(ParseError::InvalidModel(issues)),
            }
        }
        Format::Jsonld => unreachable!("normalized to Json above"),
    }
}

/// Parse raw bytes into the typed model. Transport annotations
/// (`$schema`, `@context`, `@type`) and unknown `x-*` keys are accepted and
/// ignored (SPEC.md 7.1 and 11).
pub fn parse_document(
    data: &[u8],
    format: Option<Format>,
) -> Result<(Format, SpatialDppDocument), ParseError> {
    let (fmt, value) = decode_to_value(data, format)?;
    let doc = serde_json::from_value::<SpatialDppDocument>(value)
        .map_err(|e| ParseError::Json(e.to_string()))?;
    Ok((fmt, doc))
}

// ---------------------------------------------------------------------------
// Encoders
// ---------------------------------------------------------------------------

/// Canonical JSON: sorted keys, no insignificant whitespace (SPEC.md 7.1).
pub fn to_json(doc: &SpatialDppDocument) -> Result<String, ParseError> {
    serde_json::to_string(doc).map_err(|e| ParseError::Serialize(e.to_string()))
}

pub fn to_json_pretty(doc: &SpatialDppDocument) -> Result<String, ParseError> {
    serde_json::to_string_pretty(doc).map_err(|e| ParseError::Serialize(e.to_string()))
}

/// JSON-LD: canonical JSON plus `@context` and `@type` (SPEC.md 7.2).
pub fn to_jsonld(doc: &SpatialDppDocument) -> Result<String, ParseError> {
    let mut value = serde_json::to_value(doc).map_err(|e| ParseError::Serialize(e.to_string()))?;
    if let Value::Object(map) = &mut value {
        map.insert(
            "@context".into(),
            Value::String(crate::JSONLD_CONTEXT.into()),
        );
        map.insert("@type".into(), Value::String("SpatialDppDocument".into()));
    }
    serde_json::to_string(&value).map_err(|e| ParseError::Serialize(e.to_string()))
}

/// MessagePack with struct fields as map keys (SPEC.md 7.3).
/// `to_vec_named` keeps documents self-describing so probe decodes see a
/// top-level map (rmp's default tuple form would serialize structs as arrays).
pub fn to_msgpack(doc: &SpatialDppDocument) -> Result<Vec<u8>, ParseError> {
    rmp_serde::to_vec_named(doc).map_err(|e| ParseError::Serialize(e.to_string()))
}

/// Protobuf encode never fails: the model is always representable on the wire
/// (enums are strings, so no value is out of range).
pub fn to_protobuf(doc: &SpatialDppDocument) -> Vec<u8> {
    proto::encode_document(doc)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> &'static str {
        include_str!("../../../tests/golden/valid/ev_battery_pack.json")
    }

    #[test]
    fn format_parse_and_names() {
        assert_eq!(Format::parse("JSON"), Some(Format::Json));
        assert_eq!(Format::parse("json-ld"), Some(Format::Jsonld));
        assert_eq!(Format::parse("messagepack"), Some(Format::MsgPack));
        assert_eq!(Format::parse("protobuf"), Some(Format::Protobuf));
        assert_eq!(Format::parse("yaml"), None);
    }

    #[test]
    fn detect_json() {
        let data = sample().as_bytes();
        assert_eq!(detect_format(data), Some(Format::Json));
    }

    #[test]
    fn detect_msgpack_roundtrip() {
        let (_, doc) = parse_document(sample().as_bytes(), None).expect("parse");
        let packed = to_msgpack(&doc).expect("msgpack");
        assert_eq!(detect_format(&packed), Some(Format::MsgPack));
    }

    #[test]
    fn detect_proto_roundtrip() {
        let (_, doc) = parse_document(sample().as_bytes(), None).expect("parse");
        let wire = to_protobuf(&doc);
        assert_eq!(detect_format(&wire), Some(Format::Protobuf));
    }

    #[test]
    fn garbage_is_unknown() {
        assert_eq!(detect_format(b"\x00\x01\x02"), None);
    }
}
