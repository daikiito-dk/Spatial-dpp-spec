//! Canonical data model for Spatial DPP v0.1.0 (SPEC.md Section 3).
//!
//! JSON field names are the canonical names; Protobuf and glTF representations
//! map onto them. Unknown keys (e.g. `x-*`) are permitted by the schema and
//! ignored by this implementation when parsing (SPEC.md Section 11).

use serde::{Deserialize, Serialize};

pub const SPEC_VERSION: &str = "0.1.0";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpatialDppDocument {
    pub metadata: DocumentMetadata,
    pub objects: Vec<SpatialObject>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocumentMetadata {
    pub spec_version: String,
    pub document_id: String,
    pub name: String,
    pub created: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpatialObject {
    pub id: String,
    pub label: String,
    pub geometry: GeometryRef,
    pub physical: PhysicalProperties,
    pub compliance: ComplianceRecord,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeometryRef {
    pub format: GeometryFormat,
    pub uri: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hash: Option<ContentHash>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locator: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scale_hint_m: Option<Vec<f64>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GeometryFormat {
    Gltf,
    Usda,
    Usdc,
    Step,
    Obj,
    Stl,
}

impl GeometryFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            GeometryFormat::Gltf => "gltf",
            GeometryFormat::Usda => "usda",
            GeometryFormat::Usdc => "usdc",
            GeometryFormat::Step => "step",
            GeometryFormat::Obj => "obj",
            GeometryFormat::Stl => "stl",
        }
    }

    pub fn from_wire(s: &str) -> Option<Self> {
        match s {
            "gltf" => Some(GeometryFormat::Gltf),
            "usda" => Some(GeometryFormat::Usda),
            "usdc" => Some(GeometryFormat::Usdc),
            "step" => Some(GeometryFormat::Step),
            "obj" => Some(GeometryFormat::Obj),
            "stl" => Some(GeometryFormat::Stl),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContentHash {
    pub algorithm: HashAlgorithm,
    pub value: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HashAlgorithm {
    Sha256,
    Sha512,
}

impl HashAlgorithm {
    pub fn as_str(self) -> &'static str {
        match self {
            HashAlgorithm::Sha256 => "sha256",
            HashAlgorithm::Sha512 => "sha512",
        }
    }

    pub fn from_wire(s: &str) -> Option<Self> {
        match s {
            "sha256" => Some(HashAlgorithm::Sha256),
            "sha512" => Some(HashAlgorithm::Sha512),
            _ => None,
        }
    }

    pub fn hex_len(self) -> usize {
        match self {
            HashAlgorithm::Sha256 => 64,
            HashAlgorithm::Sha512 => 128,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhysicalProperties {
    pub mass_kg: f64,
    pub center_of_mass: Vec<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub density_kg_m3: Option<f64>,
    pub friction: FrictionModel,
    pub stiffness: Stiffness,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub joints: Vec<KinematicJoint>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FrictionModel {
    /// JSON key: `static`.
    #[serde(rename = "static")]
    pub static_coefficient: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kinetic: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Stiffness {
    pub youngs_modulus_pa: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shear_modulus_pa: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub poisson_ratio: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KinematicJoint {
    pub id: String,
    /// JSON key: `type`.
    #[serde(rename = "type")]
    pub joint_type: JointType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub axis: Option<Vec<f64>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limits: Option<JointLimits>,
    pub parent: String,
    pub child: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JointType {
    Revolute,
    Prismatic,
    Fixed,
    Spherical,
}

impl JointType {
    pub fn as_str(self) -> &'static str {
        match self {
            JointType::Revolute => "revolute",
            JointType::Prismatic => "prismatic",
            JointType::Fixed => "fixed",
            JointType::Spherical => "spherical",
        }
    }

    pub fn from_wire(s: &str) -> Option<Self> {
        match s {
            "revolute" => Some(JointType::Revolute),
            "prismatic" => Some(JointType::Prismatic),
            "fixed" => Some(JointType::Fixed),
            "spherical" => Some(JointType::Spherical),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JointLimits {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lower_deg: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upper_deg: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lower_m: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upper_m: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComplianceRecord {
    pub bill_of_materials: Vec<BomEntry>,
    pub material_composition: Vec<MaterialFraction>,
    pub recyclability_rate: f64,
    pub dpp: DppRecord,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disassembly: Option<DisassemblyPlan>,
    pub cra: CraRecord,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BomEntry {
    pub part_id: String,
    pub label: String,
    pub quantity: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub material_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mass_kg: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaterialFraction {
    pub material_id: String,
    pub label: String,
    pub fraction: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DppRecord {
    pub passport_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sbom_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hbom_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repairability_score: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DisassemblyPlan {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub steps: Vec<DisassemblyStep>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DisassemblyStep {
    pub order: u32,
    pub action: DisassemblyAction,
    pub target: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hazard: Option<Hazard>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DisassemblyAction {
    Unfasten,
    Cut,
    Lift,
    Release,
    Drain,
    Deactivate,
    Other,
}

impl DisassemblyAction {
    pub fn as_str(self) -> &'static str {
        match self {
            DisassemblyAction::Unfasten => "unfasten",
            DisassemblyAction::Cut => "cut",
            DisassemblyAction::Lift => "lift",
            DisassemblyAction::Release => "release",
            DisassemblyAction::Drain => "drain",
            DisassemblyAction::Deactivate => "deactivate",
            DisassemblyAction::Other => "other",
        }
    }

    pub fn from_wire(s: &str) -> Option<Self> {
        match s {
            "unfasten" => Some(DisassemblyAction::Unfasten),
            "cut" => Some(DisassemblyAction::Cut),
            "lift" => Some(DisassemblyAction::Lift),
            "release" => Some(DisassemblyAction::Release),
            "drain" => Some(DisassemblyAction::Drain),
            "deactivate" => Some(DisassemblyAction::Deactivate),
            "other" => Some(DisassemblyAction::Other),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Hazard {
    None,
    ThermalRunaway,
    Chemical,
    Asbestos,
    Electrical,
    Sharp,
    Crush,
}

impl Hazard {
    pub fn as_str(self) -> &'static str {
        match self {
            Hazard::None => "none",
            Hazard::ThermalRunaway => "thermal_runaway",
            Hazard::Chemical => "chemical",
            Hazard::Asbestos => "asbestos",
            Hazard::Electrical => "electrical",
            Hazard::Sharp => "sharp",
            Hazard::Crush => "crush",
        }
    }

    pub fn from_wire(s: &str) -> Option<Self> {
        match s {
            "none" => Some(Hazard::None),
            "thermal_runaway" => Some(Hazard::ThermalRunaway),
            "chemical" => Some(Hazard::Chemical),
            "asbestos" => Some(Hazard::Asbestos),
            "electrical" => Some(Hazard::Electrical),
            "sharp" => Some(Hazard::Sharp),
            "crush" => Some(Hazard::Crush),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CraRecord {
    pub conformance: CraConformance,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub support_period_until: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vulnerability_disclosure_uri: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub logs: Vec<CraLogEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CraLogEntry {
    pub timestamp: String,
    pub event: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CraConformance {
    Conformant,
    Partial,
    Unknown,
}

impl CraConformance {
    pub fn as_str(self) -> &'static str {
        match self {
            CraConformance::Conformant => "conformant",
            CraConformance::Partial => "partial",
            CraConformance::Unknown => "unknown",
        }
    }

    pub fn from_wire(s: &str) -> Option<Self> {
        match s {
            "conformant" => Some(CraConformance::Conformant),
            "partial" => Some(CraConformance::Partial),
            "unknown" => Some(CraConformance::Unknown),
            _ => None,
        }
    }
}
