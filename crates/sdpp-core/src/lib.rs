//! sdpp-core: reference implementation of the Spatial DPP specification v0.1.0.
//!
//! Modules:
//! - [`model`]: canonical typed data model (mirrors `SPEC.md` Section 3)
//! - [`validate`]: structural (JSON Schema subset, `V-01`) + semantic (`V-02`+) validation
//! - [`format`]: format detection, decoding, and encoding (JSON, JSON-LD, MessagePack, Protobuf)
//! - [`proto`]: hand-maintained protobuf mirror of `schemas/v0.1.0/spatial_dpp.proto`
//! - [`gltf`]: `EXT_spatial_dpp` embedding/extraction for glTF 2.0 (`.gltf`)
//! - [`usd`]: deterministic USDA export
//! - [`inspect`]: human-readable summaries
//!
//! Determinism contract: all serializers produce sorted-key canonical output and
//! all reports are sorted by `(path, code, message)` (see AGENTS.md).

pub mod format;
pub mod gltf;
pub mod inspect;
pub mod model;
pub mod proto;
pub mod usd;
pub mod validate;

pub use format::{
    detect_format, parse_document, to_json, to_json_pretty, to_jsonld, to_msgpack, to_protobuf,
    Format, ParseError,
};
pub use model::SpatialDppDocument;
pub use validate::{
    validate_bytes, validate_document, validate_value, ValidationIssue, ValidationReport,
};

/// Specification version implemented by this crate (SPEC.md Section 11).
pub const SPEC_VERSION: &str = "0.1.0";
/// Canonical JSON Schema identifier.
pub const SCHEMA_ID: &str = "https://spatial-dpp.org/schemas/v0.1.0/spatial-dpp.schema.json";
/// Canonical JSON-LD context identifier.
pub const JSONLD_CONTEXT: &str = "https://spatial-dpp.org/contexts/v0.1.0";
