//! Hand-maintained Protobuf mirror of `schemas/v0.1.0/spatial_dpp.proto`
//! (AGENTS.md rule 1: change both files in the same commit).
//!
//! Enumerated values cross the wire as strings for forward compatibility
//! (SPEC.md 7.4). Unknown enum strings do not fail the wire decode; they
//! surface as `V-01` issues from [`proto_to_model`] so binary documents get
//! the same report shape as JSON documents.

use prost::Message;

use crate::model;
use crate::validate::ValidationIssue;

// ---------------------------------------------------------------------------
// Wire structs (mirror of schemas/v0.1.0/spatial_dpp.proto)
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq, Message)]
pub struct SpatialDppDocument {
    #[prost(message, optional, tag = "1")]
    pub metadata: Option<DocumentMetadata>,
    #[prost(message, repeated, tag = "2")]
    pub objects: Vec<SpatialObject>,
}

#[derive(Clone, PartialEq, Message)]
pub struct DocumentMetadata {
    #[prost(string, tag = "1")]
    pub spec_version: String,
    #[prost(string, tag = "2")]
    pub document_id: String,
    #[prost(string, tag = "3")]
    pub name: String,
    #[prost(string, tag = "4")]
    pub created: String,
    #[prost(string, optional, tag = "5")]
    pub author: Option<String>,
    #[prost(string, optional, tag = "6")]
    pub language: Option<String>,
}

#[derive(Clone, PartialEq, Message)]
pub struct SpatialObject {
    #[prost(string, tag = "1")]
    pub id: String,
    #[prost(string, tag = "2")]
    pub label: String,
    #[prost(message, optional, tag = "3")]
    pub geometry: Option<GeometryRef>,
    #[prost(message, optional, tag = "4")]
    pub physical: Option<PhysicalProperties>,
    #[prost(message, optional, tag = "5")]
    pub compliance: Option<ComplianceRecord>,
}

#[derive(Clone, PartialEq, Message)]
pub struct ContentHash {
    #[prost(string, tag = "1")]
    pub algorithm: String,
    #[prost(string, tag = "2")]
    pub value: String,
}

#[derive(Clone, PartialEq, Message)]
pub struct GeometryRef {
    #[prost(string, tag = "1")]
    pub format: String,
    #[prost(string, tag = "2")]
    pub uri: String,
    #[prost(message, optional, tag = "3")]
    pub hash: Option<ContentHash>,
    #[prost(string, optional, tag = "4")]
    pub locator: Option<String>,
    #[prost(double, repeated, tag = "5")]
    pub scale_hint_m: Vec<f64>,
}

#[derive(Clone, PartialEq, Message)]
pub struct FrictionModel {
    #[prost(double, tag = "1")]
    pub r#static: f64,
    #[prost(double, optional, tag = "2")]
    pub kinetic: Option<f64>,
}

#[derive(Clone, PartialEq, Message)]
pub struct Stiffness {
    #[prost(double, tag = "1")]
    pub youngs_modulus_pa: f64,
    #[prost(double, optional, tag = "2")]
    pub shear_modulus_pa: Option<f64>,
    #[prost(double, optional, tag = "3")]
    pub poisson_ratio: Option<f64>,
}

#[derive(Clone, PartialEq, Message)]
pub struct JointLimits {
    #[prost(double, optional, tag = "1")]
    pub lower_deg: Option<f64>,
    #[prost(double, optional, tag = "2")]
    pub upper_deg: Option<f64>,
    #[prost(double, optional, tag = "3")]
    pub lower_m: Option<f64>,
    #[prost(double, optional, tag = "4")]
    pub upper_m: Option<f64>,
}

#[derive(Clone, PartialEq, Message)]
pub struct KinematicJoint {
    #[prost(string, tag = "1")]
    pub id: String,
    #[prost(string, tag = "2")]
    pub r#type: String,
    #[prost(double, repeated, tag = "3")]
    pub axis: Vec<f64>,
    #[prost(message, optional, tag = "4")]
    pub limits: Option<JointLimits>,
    #[prost(string, tag = "5")]
    pub parent: String,
    #[prost(string, tag = "6")]
    pub child: String,
}

#[derive(Clone, PartialEq, Message)]
pub struct PhysicalProperties {
    #[prost(double, tag = "1")]
    pub mass_kg: f64,
    #[prost(double, repeated, tag = "2")]
    pub center_of_mass: Vec<f64>,
    #[prost(double, optional, tag = "3")]
    pub density_kg_m3: Option<f64>,
    #[prost(message, optional, tag = "4")]
    pub friction: Option<FrictionModel>,
    #[prost(message, optional, tag = "5")]
    pub stiffness: Option<Stiffness>,
    #[prost(message, repeated, tag = "6")]
    pub joints: Vec<KinematicJoint>,
}

#[derive(Clone, PartialEq, Message)]
pub struct BomEntry {
    #[prost(string, tag = "1")]
    pub part_id: String,
    #[prost(string, tag = "2")]
    pub label: String,
    #[prost(uint32, tag = "3")]
    pub quantity: u32,
    #[prost(string, optional, tag = "4")]
    pub material_id: Option<String>,
    #[prost(double, optional, tag = "5")]
    pub mass_kg: Option<f64>,
}

#[derive(Clone, PartialEq, Message)]
pub struct MaterialFraction {
    #[prost(string, tag = "1")]
    pub material_id: String,
    #[prost(string, tag = "2")]
    pub label: String,
    #[prost(double, tag = "3")]
    pub fraction: f64,
}

#[derive(Clone, PartialEq, Message)]
pub struct DppRecord {
    #[prost(string, tag = "1")]
    pub passport_id: String,
    #[prost(string, optional, tag = "2")]
    pub sbom_ref: Option<String>,
    #[prost(string, optional, tag = "3")]
    pub hbom_ref: Option<String>,
    #[prost(double, optional, tag = "4")]
    pub repairability_score: Option<f64>,
}

#[derive(Clone, PartialEq, Message)]
pub struct DisassemblyStep {
    #[prost(uint32, tag = "1")]
    pub order: u32,
    #[prost(string, tag = "2")]
    pub action: String,
    #[prost(string, tag = "3")]
    pub target: String,
    #[prost(string, optional, tag = "4")]
    pub tool: Option<String>,
    #[prost(string, optional, tag = "5")]
    pub hazard: Option<String>,
    #[prost(string, optional, tag = "6")]
    pub note: Option<String>,
}

#[derive(Clone, PartialEq, Message)]
pub struct DisassemblyPlan {
    #[prost(string, optional, tag = "1")]
    pub plan_ref: Option<String>,
    #[prost(message, repeated, tag = "2")]
    pub steps: Vec<DisassemblyStep>,
}

#[derive(Clone, PartialEq, Message)]
pub struct CraLogEntry {
    #[prost(string, tag = "1")]
    pub timestamp: String,
    #[prost(string, tag = "2")]
    pub event: String,
    #[prost(string, optional, tag = "3")]
    pub detail: Option<String>,
}

#[derive(Clone, PartialEq, Message)]
pub struct CraRecord {
    #[prost(string, tag = "1")]
    pub conformance: String,
    #[prost(string, optional, tag = "2")]
    pub support_period_until: Option<String>,
    #[prost(string, optional, tag = "3")]
    pub vulnerability_disclosure_uri: Option<String>,
    #[prost(message, repeated, tag = "4")]
    pub logs: Vec<CraLogEntry>,
}

#[derive(Clone, PartialEq, Message)]
pub struct ComplianceRecord {
    #[prost(message, repeated, tag = "1")]
    pub bill_of_materials: Vec<BomEntry>,
    #[prost(message, repeated, tag = "2")]
    pub material_composition: Vec<MaterialFraction>,
    #[prost(double, tag = "3")]
    pub recyclability_rate: f64,
    #[prost(message, optional, tag = "4")]
    pub dpp: Option<DppRecord>,
    #[prost(message, optional, tag = "5")]
    pub disassembly: Option<DisassemblyPlan>,
    #[prost(message, optional, tag = "6")]
    pub cra: Option<CraRecord>,
}

// ---------------------------------------------------------------------------
// Wire decode / encode
// ---------------------------------------------------------------------------

pub fn decode_wire(data: &[u8]) -> Result<SpatialDppDocument, prost::DecodeError> {
    SpatialDppDocument::decode(data)
}

pub fn encode_document(doc: &model::SpatialDppDocument) -> Vec<u8> {
    doc_to_wire(doc).encode_to_vec()
}

// ---------------------------------------------------------------------------
// model -> wire (infallible)
// ---------------------------------------------------------------------------

fn doc_to_wire(doc: &model::SpatialDppDocument) -> SpatialDppDocument {
    SpatialDppDocument {
        metadata: Some(metadata_to_wire(&doc.metadata)),
        objects: doc.objects.iter().map(object_to_wire).collect(),
    }
}

fn metadata_to_wire(m: &model::DocumentMetadata) -> DocumentMetadata {
    DocumentMetadata {
        spec_version: m.spec_version.clone(),
        document_id: m.document_id.clone(),
        name: m.name.clone(),
        created: m.created.clone(),
        author: m.author.clone(),
        language: m.language.clone(),
    }
}

fn object_to_wire(o: &model::SpatialObject) -> SpatialObject {
    SpatialObject {
        id: o.id.clone(),
        label: o.label.clone(),
        geometry: Some(geometry_to_wire(&o.geometry)),
        physical: Some(physical_to_wire(&o.physical)),
        compliance: Some(compliance_to_wire(&o.compliance)),
    }
}

fn geometry_to_wire(g: &model::GeometryRef) -> GeometryRef {
    GeometryRef {
        format: g.format.as_str().to_string(),
        uri: g.uri.clone(),
        hash: g.hash.as_ref().map(|h| ContentHash {
            algorithm: h.algorithm.as_str().to_string(),
            value: h.value.clone(),
        }),
        locator: g.locator.clone(),
        scale_hint_m: g.scale_hint_m.clone().unwrap_or_default(),
    }
}

fn physical_to_wire(p: &model::PhysicalProperties) -> PhysicalProperties {
    PhysicalProperties {
        mass_kg: p.mass_kg,
        center_of_mass: p.center_of_mass.clone(),
        density_kg_m3: p.density_kg_m3,
        friction: Some(FrictionModel {
            r#static: p.friction.static_coefficient,
            kinetic: p.friction.kinetic,
        }),
        stiffness: Some(Stiffness {
            youngs_modulus_pa: p.stiffness.youngs_modulus_pa,
            shear_modulus_pa: p.stiffness.shear_modulus_pa,
            poisson_ratio: p.stiffness.poisson_ratio,
        }),
        joints: p.joints.iter().map(joint_to_wire).collect(),
    }
}

fn joint_to_wire(j: &model::KinematicJoint) -> KinematicJoint {
    KinematicJoint {
        id: j.id.clone(),
        r#type: j.joint_type.as_str().to_string(),
        axis: j.axis.clone().unwrap_or_default(),
        limits: j.limits.as_ref().map(|l| JointLimits {
            lower_deg: l.lower_deg,
            upper_deg: l.upper_deg,
            lower_m: l.lower_m,
            upper_m: l.upper_m,
        }),
        parent: j.parent.clone(),
        child: j.child.clone(),
    }
}

fn compliance_to_wire(c: &model::ComplianceRecord) -> ComplianceRecord {
    ComplianceRecord {
        bill_of_materials: c.bill_of_materials.iter().map(bom_to_wire).collect(),
        material_composition: c
            .material_composition
            .iter()
            .map(fraction_to_wire)
            .collect(),
        recyclability_rate: c.recyclability_rate,
        dpp: Some(DppRecord {
            passport_id: c.dpp.passport_id.clone(),
            sbom_ref: c.dpp.sbom_ref.clone(),
            hbom_ref: c.dpp.hbom_ref.clone(),
            repairability_score: c.dpp.repairability_score,
        }),
        disassembly: c.disassembly.as_ref().map(|plan| DisassemblyPlan {
            plan_ref: plan.plan_ref.clone(),
            steps: plan
                .steps
                .iter()
                .map(|s| DisassemblyStep {
                    order: s.order,
                    action: s.action.as_str().to_string(),
                    target: s.target.clone(),
                    tool: s.tool.clone(),
                    hazard: s.hazard.map(|h| h.as_str().to_string()),
                    note: s.note.clone(),
                })
                .collect(),
        }),
        cra: Some(cra_to_wire(&c.cra)),
    }
}

fn bom_to_wire(b: &model::BomEntry) -> BomEntry {
    BomEntry {
        part_id: b.part_id.clone(),
        label: b.label.clone(),
        quantity: b.quantity,
        material_id: b.material_id.clone(),
        mass_kg: b.mass_kg,
    }
}

fn fraction_to_wire(f: &model::MaterialFraction) -> MaterialFraction {
    MaterialFraction {
        material_id: f.material_id.clone(),
        label: f.label.clone(),
        fraction: f.fraction,
    }
}

fn cra_to_wire(c: &model::CraRecord) -> CraRecord {
    CraRecord {
        conformance: c.conformance.as_str().to_string(),
        support_period_until: c.support_period_until.clone(),
        vulnerability_disclosure_uri: c.vulnerability_disclosure_uri.clone(),
        logs: c
            .logs
            .iter()
            .map(|l| CraLogEntry {
                timestamp: l.timestamp.clone(),
                event: l.event.clone(),
                detail: l.detail.clone(),
            })
            .collect(),
    }
}

// ---------------------------------------------------------------------------
// wire -> model (collects V-01 issues for structural violations)
// ---------------------------------------------------------------------------

/// Convert decoded wire data into the typed model. Structural violations that
/// cannot be represented in the model (unknown enum strings, missing required
/// messages) return `Err(issues)`; every other constraint is left to the
/// semantic layer, so binary documents report the same codes as JSON.
pub fn proto_to_model(
    wire: &SpatialDppDocument,
) -> Result<model::SpatialDppDocument, Vec<ValidationIssue>> {
    let mut issues = Vec::new();
    let metadata = match wire.metadata.as_ref() {
        Some(m) => model::DocumentMetadata {
            spec_version: m.spec_version.clone(),
            document_id: m.document_id.clone(),
            name: m.name.clone(),
            created: m.created.clone(),
            author: m.author.clone(),
            language: m.language.clone(),
        },
        None => {
            issues.push(ValidationIssue::new(
                "V-01",
                "/metadata",
                "protobuf document is missing the required `metadata` message",
            ));
            model::DocumentMetadata {
                spec_version: String::new(),
                document_id: String::new(),
                name: String::new(),
                created: String::new(),
                author: None,
                language: None,
            }
        }
    };
    let objects = wire
        .objects
        .iter()
        .enumerate()
        .map(|(i, o)| object_from_wire(o, &format!("/objects/{i}"), &mut issues))
        .collect();
    if issues.is_empty() {
        Ok(model::SpatialDppDocument { metadata, objects })
    } else {
        Err(issues)
    }
}

fn object_from_wire(
    o: &SpatialObject,
    base: &str,
    issues: &mut Vec<ValidationIssue>,
) -> model::SpatialObject {
    let geometry = match o.geometry.as_ref() {
        Some(g) => geometry_from_wire(g, &format!("{base}/geometry"), issues),
        None => {
            issues.push(ValidationIssue::new(
                "V-01",
                format!("{base}/geometry"),
                "protobuf object is missing the required `geometry` message",
            ));
            model::GeometryRef {
                format: model::GeometryFormat::Gltf,
                uri: String::new(),
                hash: None,
                locator: None,
                scale_hint_m: None,
            }
        }
    };
    let physical = match o.physical.as_ref() {
        Some(p) => physical_from_wire(p, &format!("{base}/physical"), issues),
        None => {
            issues.push(ValidationIssue::new(
                "V-01",
                format!("{base}/physical"),
                "protobuf object is missing the required `physical` message",
            ));
            model::PhysicalProperties {
                mass_kg: 0.0,
                center_of_mass: Vec::new(),
                density_kg_m3: None,
                friction: model::FrictionModel {
                    static_coefficient: 0.0,
                    kinetic: None,
                },
                stiffness: model::Stiffness {
                    youngs_modulus_pa: 0.0,
                    shear_modulus_pa: None,
                    poisson_ratio: None,
                },
                joints: Vec::new(),
            }
        }
    };
    let compliance = match o.compliance.as_ref() {
        Some(c) => compliance_from_wire(c, &format!("{base}/compliance"), issues),
        None => {
            issues.push(ValidationIssue::new(
                "V-01",
                format!("{base}/compliance"),
                "protobuf object is missing the required `compliance` message",
            ));
            model::ComplianceRecord {
                bill_of_materials: Vec::new(),
                material_composition: Vec::new(),
                recyclability_rate: 0.0,
                dpp: model::DppRecord {
                    passport_id: String::new(),
                    sbom_ref: None,
                    hbom_ref: None,
                    repairability_score: None,
                },
                disassembly: None,
                cra: model::CraRecord {
                    conformance: model::CraConformance::Conformant,
                    support_period_until: None,
                    vulnerability_disclosure_uri: None,
                    logs: Vec::new(),
                },
            }
        }
    };
    model::SpatialObject {
        id: o.id.clone(),
        label: o.label.clone(),
        geometry,
        physical,
        compliance,
    }
}

fn geometry_from_wire(
    g: &GeometryRef,
    base: &str,
    issues: &mut Vec<ValidationIssue>,
) -> model::GeometryRef {
    let format = match model::GeometryFormat::from_wire(&g.format) {
        Some(f) => f,
        None => {
            issues.push(ValidationIssue::new(
                "V-01",
                format!("{base}/format"),
                format!("unknown geometry format `{}`", g.format),
            ));
            model::GeometryFormat::Gltf
        }
    };
    let hash = g.hash.as_ref().map(|h| {
        let algorithm = match model::HashAlgorithm::from_wire(&h.algorithm) {
            Some(a) => a,
            None => {
                issues.push(ValidationIssue::new(
                    "V-01",
                    format!("{base}/hash/algorithm"),
                    format!("unknown hash algorithm `{}`", h.algorithm),
                ));
                model::HashAlgorithm::Sha256
            }
        };
        model::ContentHash {
            algorithm,
            value: h.value.clone(),
        }
    });
    model::GeometryRef {
        format,
        uri: g.uri.clone(),
        hash,
        locator: g.locator.clone(),
        scale_hint_m: if g.scale_hint_m.is_empty() {
            None
        } else {
            Some(g.scale_hint_m.clone())
        },
    }
}

fn physical_from_wire(
    p: &PhysicalProperties,
    base: &str,
    issues: &mut Vec<ValidationIssue>,
) -> model::PhysicalProperties {
    let friction = match p.friction.as_ref() {
        Some(f) => model::FrictionModel {
            static_coefficient: f.r#static,
            kinetic: f.kinetic,
        },
        None => {
            issues.push(ValidationIssue::new(
                "V-01",
                format!("{base}/friction"),
                "protobuf physical properties are missing the required `friction` message",
            ));
            model::FrictionModel {
                static_coefficient: 0.0,
                kinetic: None,
            }
        }
    };
    let stiffness = match p.stiffness.as_ref() {
        Some(s) => model::Stiffness {
            youngs_modulus_pa: s.youngs_modulus_pa,
            shear_modulus_pa: s.shear_modulus_pa,
            poisson_ratio: s.poisson_ratio,
        },
        None => {
            issues.push(ValidationIssue::new(
                "V-01",
                format!("{base}/stiffness"),
                "protobuf physical properties are missing the required `stiffness` message",
            ));
            model::Stiffness {
                youngs_modulus_pa: 0.0,
                shear_modulus_pa: None,
                poisson_ratio: None,
            }
        }
    };
    let joints = p
        .joints
        .iter()
        .enumerate()
        .map(|(j, joint)| {
            let jbase = format!("{base}/joints/{j}");
            let joint_type = match model::JointType::from_wire(&joint.r#type) {
                Some(t) => t,
                None => {
                    issues.push(ValidationIssue::new(
                        "V-01",
                        format!("{jbase}/type"),
                        format!("unknown joint type `{}`", joint.r#type),
                    ));
                    model::JointType::Fixed
                }
            };
            model::KinematicJoint {
                id: joint.id.clone(),
                joint_type,
                axis: if joint.axis.is_empty() {
                    None
                } else {
                    Some(joint.axis.clone())
                },
                limits: joint.limits.as_ref().map(|l| model::JointLimits {
                    lower_deg: l.lower_deg,
                    upper_deg: l.upper_deg,
                    lower_m: l.lower_m,
                    upper_m: l.upper_m,
                }),
                parent: joint.parent.clone(),
                child: joint.child.clone(),
            }
        })
        .collect();
    model::PhysicalProperties {
        mass_kg: p.mass_kg,
        center_of_mass: p.center_of_mass.clone(),
        density_kg_m3: p.density_kg_m3,
        friction,
        stiffness,
        joints,
    }
}

fn compliance_from_wire(
    c: &ComplianceRecord,
    base: &str,
    issues: &mut Vec<ValidationIssue>,
) -> model::ComplianceRecord {
    let dpp = match c.dpp.as_ref() {
        Some(d) => model::DppRecord {
            passport_id: d.passport_id.clone(),
            sbom_ref: d.sbom_ref.clone(),
            hbom_ref: d.hbom_ref.clone(),
            repairability_score: d.repairability_score,
        },
        None => {
            issues.push(ValidationIssue::new(
                "V-01",
                format!("{base}/dpp"),
                "protobuf compliance record is missing the required `dpp` message",
            ));
            model::DppRecord {
                passport_id: String::new(),
                sbom_ref: None,
                hbom_ref: None,
                repairability_score: None,
            }
        }
    };
    let cra = match c.cra.as_ref() {
        Some(r) => {
            let conformance = match model::CraConformance::from_wire(&r.conformance) {
                Some(v) => v,
                None => {
                    issues.push(ValidationIssue::new(
                        "V-01",
                        format!("{base}/cra/conformance"),
                        format!("unknown CRA conformance `{}`", r.conformance),
                    ));
                    model::CraConformance::Conformant
                }
            };
            model::CraRecord {
                conformance,
                support_period_until: r.support_period_until.clone(),
                vulnerability_disclosure_uri: r.vulnerability_disclosure_uri.clone(),
                logs: r
                    .logs
                    .iter()
                    .map(|l| model::CraLogEntry {
                        timestamp: l.timestamp.clone(),
                        event: l.event.clone(),
                        detail: l.detail.clone(),
                    })
                    .collect(),
            }
        }
        None => {
            issues.push(ValidationIssue::new(
                "V-01",
                format!("{base}/cra"),
                "protobuf compliance record is missing the required `cra` message",
            ));
            model::CraRecord {
                conformance: model::CraConformance::Conformant,
                support_period_until: None,
                vulnerability_disclosure_uri: None,
                logs: Vec::new(),
            }
        }
    };
    let disassembly = c.disassembly.as_ref().map(|plan| model::DisassemblyPlan {
        plan_ref: plan.plan_ref.clone(),
        steps: plan
            .steps
            .iter()
            .enumerate()
            .map(|(s, step)| {
                let sbase = format!("{base}/disassembly/steps/{s}");
                let action = match model::DisassemblyAction::from_wire(&step.action) {
                    Some(a) => a,
                    None => {
                        issues.push(ValidationIssue::new(
                            "V-01",
                            format!("{sbase}/action"),
                            format!("unknown disassembly action `{}`", step.action),
                        ));
                        model::DisassemblyAction::Other
                    }
                };
                let hazard = step
                    .hazard
                    .as_deref()
                    .map(|h| match model::Hazard::from_wire(h) {
                        Some(v) => v,
                        None => {
                            issues.push(ValidationIssue::new(
                                "V-01",
                                format!("{sbase}/hazard"),
                                format!("unknown disassembly hazard `{h}`"),
                            ));
                            model::Hazard::None
                        }
                    });
                model::DisassemblyStep {
                    order: step.order,
                    action,
                    target: step.target.clone(),
                    tool: step.tool.clone(),
                    hazard,
                    note: step.note.clone(),
                }
            })
            .collect(),
    });
    model::ComplianceRecord {
        bill_of_materials: c
            .bill_of_materials
            .iter()
            .map(|b| model::BomEntry {
                part_id: b.part_id.clone(),
                label: b.label.clone(),
                quantity: b.quantity,
                material_id: b.material_id.clone(),
                mass_kg: b.mass_kg,
            })
            .collect(),
        material_composition: c
            .material_composition
            .iter()
            .map(|f| model::MaterialFraction {
                material_id: f.material_id.clone(),
                label: f.label.clone(),
                fraction: f.fraction,
            })
            .collect(),
        recyclability_rate: c.recyclability_rate,
        dpp,
        disassembly,
        cra,
    }
}
