//! Structural (`V-01`) and semantic (`V-02`+) validation (SPEC.md Section 6).
//!
//! Two layers:
//! 1. A structural subset of the published JSON Schema (`type`, `required`,
//!    `enum`, `const`, `items`, `additionalProperties: false`, `$ref`).
//! 2. A semantic layer that enforces every value-level constraint with its own
//!    `V-xx` code, including the `V-01` twins of structural rules so binary
//!    formats (MessagePack, Protobuf) cannot bypass them.

use std::collections::HashSet;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::format::{decode_to_value, Format, ParseError};
use crate::model::{self, JointType, SpatialDppDocument};

/// The canonical schema, embedded at compile time (AGENTS.md rule 1).
pub const EMBEDDED_SCHEMA: &str = include_str!("../../../schemas/v0.1.0/spatial-dpp.schema.json");

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValidationIssue {
    pub code: String,
    pub path: String,
    pub message: String,
}

impl ValidationIssue {
    pub fn new(code: &str, path: impl Into<String>, message: impl Into<String>) -> Self {
        ValidationIssue {
            code: code.to_string(),
            path: path.into(),
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValidationReport {
    pub valid: bool,
    pub errors: Vec<ValidationIssue>,
    pub warnings: Vec<ValidationIssue>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document_id: Option<String>,
    pub object_count: usize,
}

// ---------------------------------------------------------------------------
// Structural layer: JSON Schema subset
// ---------------------------------------------------------------------------

fn schema_root() -> &'static Value {
    static ROOT: OnceLock<Value> = OnceLock::new();
    // A malformed embedded schema is a build bug, never user input; the test
    // suite asserts parseability.
    ROOT.get_or_init(|| {
        serde_json::from_str(EMBEDDED_SCHEMA).expect("embedded JSON Schema must parse")
    })
}

/// Enforce the structural subset of the embedded schema (reported as `V-01`).
pub fn schema_subset_issues(value: &Value) -> Vec<ValidationIssue> {
    let root = schema_root();
    let mut issues = Vec::new();
    check_value(root, root, value, "", &mut issues);
    issues
}

fn check_value(
    root: &Value,
    schema: &Value,
    instance: &Value,
    path: &str,
    issues: &mut Vec<ValidationIssue>,
) {
    let Some(s) = schema.as_object() else { return };

    if let Some(Value::String(reference)) = s.get("$ref") {
        if let Some(target) = resolve_pointer(root, reference) {
            check_value(root, target, instance, path, issues);
        }
        return;
    }

    if let Some(types) = s.get("type") {
        let allowed: Vec<&str> = match types {
            Value::String(t) => vec![t.as_str()],
            Value::Array(a) => a.iter().filter_map(Value::as_str).collect(),
            _ => Vec::new(),
        };
        if !allowed.is_empty() && !allowed.iter().any(|t| type_matches(t, instance)) {
            issues.push(ValidationIssue::new(
                "V-01",
                path,
                format!(
                    "expected type {} but found {}",
                    allowed.join("|"),
                    actual_type(instance)
                ),
            ));
        }
    }

    if let (Some(Value::Array(required)), Some(map)) = (s.get("required"), instance.as_object()) {
        for key in required.iter().filter_map(Value::as_str) {
            if !map.contains_key(key) {
                issues.push(ValidationIssue::new(
                    "V-01",
                    path,
                    format!("missing required property `{key}`"),
                ));
            }
        }
    }

    if let Some(Value::Array(allowed)) = s.get("enum") {
        if !allowed.is_empty() && !allowed.contains(instance) {
            issues.push(ValidationIssue::new(
                "V-01",
                path,
                format!(
                    "value {} is not one of the allowed values",
                    serde_json::to_string(instance).unwrap_or_else(|_| "<unrepresentable>".into())
                ),
            ));
        }
    }

    if let Some(expected) = s.get("const") {
        if instance != expected {
            issues.push(ValidationIssue::new(
                "V-01",
                path,
                format!(
                    "must equal {}",
                    serde_json::to_string(expected).unwrap_or_else(|_| "<unrepresentable>".into())
                ),
            ));
        }
    }

    if let (Some(Value::Object(props)), Some(map)) = (s.get("properties"), instance.as_object()) {
        for (key, subschema) in props {
            if let Some(child) = map.get(key) {
                check_value(root, subschema, child, &format!("{path}/{key}"), issues);
            }
        }
    }

    // Closed key sets (`additionalProperties: false`, used by jointLimits).
    if let (Some(Value::Bool(false)), Some(map)) =
        (s.get("additionalProperties"), instance.as_object())
    {
        if let Some(Value::Object(props)) = s.get("properties") {
            for key in map.keys() {
                if !props.contains_key(key) {
                    issues.push(ValidationIssue::new(
                        "V-01",
                        path,
                        format!("property `{key}` is not allowed here"),
                    ));
                }
            }
        }
    }

    if let (Some(items), Some(array)) = (s.get("items"), instance.as_array()) {
        for (i, item) in array.iter().enumerate() {
            check_value(root, items, item, &format!("{path}/{i}"), issues);
        }
    }
}

fn resolve_pointer<'a>(root: &'a Value, reference: &str) -> Option<&'a Value> {
    let rel = reference.strip_prefix("#/")?;
    let mut current = root;
    for segment in rel.split('/') {
        current = match current {
            Value::Object(map) => map.get(segment)?,
            Value::Array(array) => array.get(segment.parse::<usize>().ok()?)?,
            _ => return None,
        };
    }
    Some(current)
}

fn type_matches(expected: &str, value: &Value) -> bool {
    match expected {
        "object" => value.is_object(),
        "array" => value.is_array(),
        "string" => value.is_string(),
        "number" => value.is_number(),
        // Strict integers: a JSON `3.0` is rejected so the canonical form keeps
        // integers as integers (documented deviation from draft 2020-12).
        "integer" => value.is_i64() || value.is_u64(),
        "boolean" => value.is_boolean(),
        "null" => value.is_null(),
        _ => true,
    }
}

fn actual_type(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(n) if n.is_i64() || n.is_u64() => "integer",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

// ---------------------------------------------------------------------------
// Semantic layer
// ---------------------------------------------------------------------------

/// Run every semantic rule (`V-01` twins plus `V-02`…`V-17`) and warnings.
pub fn semantic_issues(doc: &SpatialDppDocument) -> (Vec<ValidationIssue>, Vec<ValidationIssue>) {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    // --- V-01 twins (structural rules for binary formats) ---
    if doc.objects.is_empty() {
        errors.push(ValidationIssue::new(
            "V-01",
            "/objects",
            "document must contain at least 1 object",
        ));
    }
    if doc.metadata.spec_version != model::SPEC_VERSION {
        errors.push(ValidationIssue::new(
            "V-01",
            "/metadata/spec_version",
            format!(
                "must equal \"{}\" (got `{}`)",
                model::SPEC_VERSION,
                doc.metadata.spec_version
            ),
        ));
    }

    // --- metadata ---
    non_empty(
        &mut errors,
        "V-17",
        "/metadata/name",
        &doc.metadata.name,
        "metadata.name",
    );
    check_uri(
        &mut errors,
        "V-16",
        "/metadata/document_id",
        &doc.metadata.document_id,
        "document_id",
    );
    check_rfc3339(
        &mut errors,
        "V-14",
        "/metadata/created",
        &doc.metadata.created,
        "created",
    );

    // --- object id index for V-07 / V-08 / V-12 ---
    let mut object_ids: HashSet<&str> = HashSet::new();
    for (i, obj) in doc.objects.iter().enumerate() {
        if !object_ids.insert(obj.id.as_str()) {
            errors.push(ValidationIssue::new(
                "V-07",
                format!("/objects/{i}"),
                format!("duplicate object id `{}`", obj.id),
            ));
        }
    }

    for (i, obj) in doc.objects.iter().enumerate() {
        check_object(&mut errors, &mut warnings, i, obj, &object_ids);
    }

    (errors, warnings)
}

fn check_object(
    errors: &mut Vec<ValidationIssue>,
    warnings: &mut Vec<ValidationIssue>,
    i: usize,
    obj: &model::SpatialObject,
    object_ids: &HashSet<&str>,
) {
    let base = format!("/objects/{i}");
    non_empty(errors, "V-17", format!("{base}/id"), &obj.id, "id");
    non_empty(errors, "V-17", format!("{base}/label"), &obj.label, "label");

    // --- geometry (V-13, V-15, V-16, W-01) ---
    let gbase = format!("{base}/geometry");
    let g = &obj.geometry;
    check_uri(
        errors,
        "V-16",
        format!("{gbase}/uri"),
        &g.uri,
        "geometry.uri",
    );
    match &g.hash {
        Some(h) => {
            let expected_len = h.algorithm.hex_len();
            if h.value.len() != expected_len || !is_lowercase_hex(&h.value) {
                errors.push(ValidationIssue::new(
                    "V-13",
                    format!("{gbase}/hash/value"),
                    format!(
                        "hash value for {} must be {} lowercase hex characters (got `{}`)",
                        h.algorithm.as_str(),
                        expected_len,
                        h.value
                    ),
                ));
            }
        }
        None => warnings.push(ValidationIssue::new(
            "W-01",
            gbase.clone(),
            "geometry has no content hash; asset integrity cannot be verified",
        )),
    }
    if let Some(scale) = &g.scale_hint_m {
        if scale.len() != 3 {
            errors.push(ValidationIssue::new(
                "V-15",
                format!("{gbase}/scale_hint_m"),
                format!(
                    "scale_hint_m must have exactly 3 entries (got {})",
                    scale.len()
                ),
            ));
        } else if let Some(bad) = scale.iter().find(|v| !v.is_finite() || **v <= 0.0) {
            errors.push(ValidationIssue::new(
                "V-15",
                format!("{gbase}/scale_hint_m"),
                format!("scale_hint_m entries must be > 0 (got {bad})"),
            ));
        }
    }

    // --- physical (V-02, V-03, V-04, V-05) ---
    let pbase = format!("{base}/physical");
    let p = &obj.physical;
    if !p.mass_kg.is_finite() || p.mass_kg <= 0.0 {
        errors.push(ValidationIssue::new(
            "V-02",
            format!("{pbase}/mass_kg"),
            format!("mass_kg must be finite and > 0 (got {})", p.mass_kg),
        ));
    }
    if p.center_of_mass.len() != 3 {
        errors.push(ValidationIssue::new(
            "V-05",
            format!("{pbase}/center_of_mass"),
            format!(
                "center_of_mass must have exactly 3 coordinates (got {})",
                p.center_of_mass.len()
            ),
        ));
    }
    for (j, c) in p.center_of_mass.iter().enumerate() {
        if !c.is_finite() || *c < 0.0 || *c > 1.0 {
            errors.push(ValidationIssue::new(
                "V-05",
                format!("{pbase}/center_of_mass/{j}"),
                format!("center_of_mass[{j}] must be within [0,1] (got {c})"),
            ));
        }
    }
    if !p.friction.static_coefficient.is_finite() || p.friction.static_coefficient < 0.0 {
        errors.push(ValidationIssue::new(
            "V-03",
            format!("{pbase}/friction/static"),
            format!(
                "friction.static must be finite and >= 0 (got {})",
                p.friction.static_coefficient
            ),
        ));
    }
    if let Some(k) = p.friction.kinetic {
        if !k.is_finite() || k < 0.0 {
            errors.push(ValidationIssue::new(
                "V-03",
                format!("{pbase}/friction/kinetic"),
                format!("friction.kinetic must be finite and >= 0 (got {k})"),
            ));
        }
    }
    let s = &p.stiffness;
    if !s.youngs_modulus_pa.is_finite() || s.youngs_modulus_pa <= 0.0 {
        errors.push(ValidationIssue::new(
            "V-04",
            format!("{pbase}/stiffness/youngs_modulus_pa"),
            format!(
                "youngs_modulus_pa must be finite and > 0 (got {})",
                s.youngs_modulus_pa
            ),
        ));
    }
    if let Some(v) = s.shear_modulus_pa {
        if !v.is_finite() || v <= 0.0 {
            errors.push(ValidationIssue::new(
                "V-04",
                format!("{pbase}/stiffness/shear_modulus_pa"),
                format!("shear_modulus_pa must be finite and > 0 (got {v})"),
            ));
        }
    }
    if let Some(v) = s.poisson_ratio {
        if !v.is_finite() || !(0.0..0.5).contains(&v) {
            errors.push(ValidationIssue::new(
                "V-04",
                format!("{pbase}/stiffness/poisson_ratio"),
                format!("poisson_ratio must be within [0, 0.5) (got {v})"),
            ));
        }
    }
    if let Some(v) = p.density_kg_m3 {
        if !v.is_finite() || v <= 0.0 {
            errors.push(ValidationIssue::new(
                "V-04",
                format!("{pbase}/density_kg_m3"),
                format!("density_kg_m3 must be finite and > 0 (got {v})"),
            ));
        }
    }

    // --- joints (V-06, V-07, V-08, V-17) ---
    let mut joint_ids: HashSet<&str> = HashSet::new();
    for (j, joint) in p.joints.iter().enumerate() {
        let jbase = format!("{pbase}/joints/{j}");
        non_empty(errors, "V-17", format!("{jbase}/id"), &joint.id, "joint.id");
        if !joint_ids.insert(joint.id.as_str()) {
            errors.push(ValidationIssue::new(
                "V-07",
                jbase.clone(),
                format!("duplicate joint id `{}`", joint.id),
            ));
        }
        let type_str = joint.joint_type.as_str();
        if joint.joint_type != JointType::Fixed {
            match &joint.axis {
                None => errors.push(ValidationIssue::new(
                    "V-06",
                    jbase.clone(),
                    format!("joint `{}` requires an `axis`", joint.id),
                )),
                Some(axis) => {
                    if axis.len() != 3 {
                        errors.push(ValidationIssue::new(
                            "V-06",
                            format!("{jbase}/axis"),
                            format!(
                                "joint `{}` axis must have exactly 3 elements (got {})",
                                joint.id,
                                axis.len()
                            ),
                        ));
                    } else {
                        let norm =
                            (axis[0] * axis[0] + axis[1] * axis[1] + axis[2] * axis[2]).sqrt();
                        if !norm.is_finite() || (norm - 1.0).abs() > 1e-6 {
                            errors.push(ValidationIssue::new(
                                "V-06",
                                format!("{jbase}/axis"),
                                format!(
                                    "joint `{}` axis must be a unit vector within 1e-6 (length {})",
                                    joint.id, norm
                                ),
                            ));
                        }
                    }
                }
            }
        }
        match joint.joint_type {
            JointType::Prismatic => match &joint.limits {
                None => errors.push(ValidationIssue::new(
                    "V-06",
                    jbase.clone(),
                    format!("joint `{}` of type prismatic requires `limits`", joint.id),
                )),
                Some(l) => check_joint_limits(errors, &jbase, &joint.id, type_str, l, false),
            },
            JointType::Revolute | JointType::Spherical => {
                if let Some(l) = &joint.limits {
                    check_joint_limits(errors, &jbase, &joint.id, type_str, l, true);
                }
            }
            JointType::Fixed => {}
        }
        if joint.parent == joint.child {
            errors.push(ValidationIssue::new(
                "V-08",
                jbase.clone(),
                format!(
                    "joint `{}` parent and child must differ (both `{}`)",
                    joint.id, joint.parent
                ),
            ));
        }
        if !object_ids.contains(joint.parent.as_str()) {
            errors.push(ValidationIssue::new(
                "V-08",
                format!("{jbase}/parent"),
                format!(
                    "joint `{}` parent `{}` does not resolve to an object id in this document",
                    joint.id, joint.parent
                ),
            ));
        }
        if !object_ids.contains(joint.child.as_str()) {
            errors.push(ValidationIssue::new(
                "V-08",
                format!("{jbase}/child"),
                format!(
                    "joint `{}` child `{}` does not resolve to an object id in this document",
                    joint.id, joint.child
                ),
            ));
        }
    }

    // --- compliance ---
    check_compliance(errors, warnings, &base, obj, object_ids);
}

/// `deg_keys = true` → degree limits (revolute/spherical); `false` → meter limits (prismatic).
fn check_joint_limits(
    errors: &mut Vec<ValidationIssue>,
    jbase: &str,
    joint_id: &str,
    type_str: &str,
    l: &model::JointLimits,
    deg_keys: bool,
) {
    let (good, bad, lower, upper): (&str, &str, Option<f64>, Option<f64>) = if deg_keys {
        ("lower_deg", "upper_deg", l.lower_deg, l.upper_deg)
    } else {
        ("lower_m", "upper_m", l.lower_m, l.upper_m)
    };
    let wrong_keys: [(String, bool); 4] = [
        ("lower_deg".to_string(), l.lower_deg.is_some()),
        ("upper_deg".to_string(), l.upper_deg.is_some()),
        ("lower_m".to_string(), l.lower_m.is_some()),
        ("upper_m".to_string(), l.upper_m.is_some()),
    ];
    for (key, present) in wrong_keys {
        let is_good_key = key == good || key == bad;
        if present && !is_good_key {
            errors.push(ValidationIssue::new(
                "V-06",
                format!("{jbase}/limits/{key}"),
                format!(
                    "joint `{}` limits key `{key}` is not valid for type {type_str}",
                    joint_id
                ),
            ));
        }
    }
    match (lower, upper) {
        (Some(lo), Some(hi)) => {
            if !lo.is_finite() || !hi.is_finite() || lo > hi {
                errors.push(ValidationIssue::new(
                    "V-06",
                    format!("{jbase}/limits"),
                    format!("joint `{joint_id}` lower limit {lo} must be <= upper limit {hi}"),
                ));
            }
        }
        _ => errors.push(ValidationIssue::new(
            "V-06",
            format!("{jbase}/limits"),
            format!("joint `{joint_id}` limits for type {type_str} require `{good}` and `{bad}`"),
        )),
    }
}

fn check_compliance(
    errors: &mut Vec<ValidationIssue>,
    warnings: &mut Vec<ValidationIssue>,
    base: &str,
    obj: &model::SpatialObject,
    object_ids: &HashSet<&str>,
) {
    let cbase = format!("{base}/compliance");
    let c = &obj.compliance;

    // V-01 twins
    if c.bill_of_materials.is_empty() {
        errors.push(ValidationIssue::new(
            "V-01",
            format!("{cbase}/bill_of_materials"),
            "bill_of_materials must contain at least 1 entry",
        ));
    }
    if c.material_composition.is_empty() {
        errors.push(ValidationIssue::new(
            "V-01",
            format!("{cbase}/material_composition"),
            "material_composition must contain at least 1 entry",
        ));
    }

    // material composition (V-07, V-09, V-17)
    let mut material_ids: HashSet<&str> = HashSet::new();
    let mut sum = 0.0;
    for (m, frac) in c.material_composition.iter().enumerate() {
        let mbase = format!("{cbase}/material_composition/{m}");
        non_empty(
            errors,
            "V-17",
            format!("{mbase}/material_id"),
            &frac.material_id,
            "material_id",
        );
        non_empty(
            errors,
            "V-17",
            format!("{mbase}/label"),
            &frac.label,
            "material.label",
        );
        if !material_ids.insert(frac.material_id.as_str()) {
            errors.push(ValidationIssue::new(
                "V-07",
                mbase.clone(),
                format!("duplicate material_id `{}`", frac.material_id),
            ));
        }
        if !frac.fraction.is_finite() || frac.fraction <= 0.0 || frac.fraction > 1.0 {
            errors.push(ValidationIssue::new(
                "V-09",
                format!("{mbase}/fraction"),
                format!(
                    "material fraction for `{}` must be in (0, 1] (got {})",
                    frac.material_id, frac.fraction
                ),
            ));
        }
        sum += frac.fraction;
    }
    if !c.material_composition.is_empty() && (!sum.is_finite() || (sum - 1.0).abs() > 0.001) {
        errors.push(ValidationIssue::new(
            "V-09",
            format!("{cbase}/material_composition"),
            format!("material fractions must sum to 1 within ±0.001 (got {sum})"),
        ));
    }

    // BOM (V-02, V-07, V-10, V-15, V-17)
    let mut part_ids: HashSet<&str> = HashSet::new();
    for (b, entry) in c.bill_of_materials.iter().enumerate() {
        let bbase = format!("{cbase}/bill_of_materials/{b}");
        non_empty(
            errors,
            "V-17",
            format!("{bbase}/part_id"),
            &entry.part_id,
            "part_id",
        );
        non_empty(
            errors,
            "V-17",
            format!("{bbase}/label"),
            &entry.label,
            "part.label",
        );
        if !part_ids.insert(entry.part_id.as_str()) {
            errors.push(ValidationIssue::new(
                "V-07",
                bbase.clone(),
                format!("duplicate part_id `{}`", entry.part_id),
            ));
        }
        if entry.quantity < 1 {
            errors.push(ValidationIssue::new(
                "V-15",
                format!("{bbase}/quantity"),
                format!("quantity must be >= 1 (got {})", entry.quantity),
            ));
        }
        if let Some(mid) = &entry.material_id {
            if !material_ids.contains(mid.as_str()) {
                errors.push(ValidationIssue::new(
                    "V-10",
                    format!("{bbase}/material_id"),
                    format!(
                        "BOM entry `{}` references unknown material_id `{}`",
                        entry.part_id, mid
                    ),
                ));
            }
        }
        if let Some(mass) = entry.mass_kg {
            if !mass.is_finite() || mass <= 0.0 {
                errors.push(ValidationIssue::new(
                    "V-02",
                    format!("{bbase}/mass_kg"),
                    format!("BOM mass_kg must be finite and > 0 (got {mass})"),
                ));
            }
        }
    }

    // rates (V-11)
    if !c.recyclability_rate.is_finite() || c.recyclability_rate < 0.0 || c.recyclability_rate > 1.0
    {
        errors.push(ValidationIssue::new(
            "V-11",
            format!("{cbase}/recyclability_rate"),
            format!(
                "recyclability_rate must be within [0, 1] (got {})",
                c.recyclability_rate
            ),
        ));
    }
    if let Some(score) = c.dpp.repairability_score {
        if !score.is_finite() || !(0.0..=1.0).contains(&score) {
            errors.push(ValidationIssue::new(
                "V-11",
                format!("{cbase}/dpp/repairability_score"),
                format!("repairability_score must be within [0, 1] (got {score})"),
            ));
        }
    }

    // DPP record (V-16, V-17)
    non_empty(
        errors,
        "V-17",
        format!("{cbase}/dpp/passport_id"),
        &c.dpp.passport_id,
        "passport_id",
    );
    if let Some(uri) = &c.dpp.sbom_ref {
        check_uri(
            errors,
            "V-16",
            format!("{cbase}/dpp/sbom_ref"),
            uri,
            "sbom_ref",
        );
    }
    if let Some(uri) = &c.dpp.hbom_ref {
        check_uri(
            errors,
            "V-16",
            format!("{cbase}/dpp/hbom_ref"),
            uri,
            "hbom_ref",
        );
    }

    // disassembly (V-12, V-16, W-03)
    match &c.disassembly {
        None => warnings.push(ValidationIssue::new(
            "W-03",
            format!("{cbase}/disassembly"),
            "compliance has no disassembly plan; rescue/recycling robots will lack machine-readable take-apart guidance",
        )),
        Some(plan) => {
            if let Some(uri) = &plan.plan_ref {
                check_uri(errors, "V-16", format!("{cbase}/disassembly/plan_ref"), uri, "plan_ref");
            }
            for (s, step) in plan.steps.iter().enumerate() {
                let sbase = format!("{cbase}/disassembly/steps/{s}");
                if step.order != (s as u32) + 1 {
                    errors.push(ValidationIssue::new(
                        "V-12",
                        format!("{sbase}/order"),
                        format!("disassembly step order must be {}, got {}", s + 1, step.order),
                    ));
                }
                if !part_ids.contains(step.target.as_str()) && !object_ids.contains(step.target.as_str()) {
                    errors.push(ValidationIssue::new(
                        "V-12",
                        format!("{sbase}/target"),
                        format!(
                            "disassembly step {} target `{}` does not resolve to a part_id or object id",
                            step.order, step.target
                        ),
                    ));
                }
            }
        }
    }

    // CRA record (V-14, V-16, V-17, W-02)
    let cra = &c.cra;
    if let Some(ts) = &cra.support_period_until {
        check_rfc3339(
            errors,
            "V-14",
            format!("{cbase}/cra/support_period_until"),
            ts,
            "cra.support_period_until",
        );
    }
    if let Some(uri) = &cra.vulnerability_disclosure_uri {
        check_uri(
            errors,
            "V-16",
            format!("{cbase}/cra/vulnerability_disclosure_uri"),
            uri,
            "vulnerability_disclosure_uri",
        );
    }
    for (l, log) in cra.logs.iter().enumerate() {
        let lbase = format!("{cbase}/cra/logs/{l}");
        check_rfc3339(
            errors,
            "V-14",
            format!("{lbase}/timestamp"),
            &log.timestamp,
            "cra.logs[].timestamp",
        );
        non_empty(
            errors,
            "V-17",
            format!("{lbase}/event"),
            &log.event,
            "cra.logs[].event",
        );
    }
    if cra.conformance == model::CraConformance::Unknown {
        warnings.push(ValidationIssue::new(
            "W-02",
            format!("{cbase}/cra/conformance"),
            "CRA conformance is `unknown`",
        ));
    }
}

// ---------------------------------------------------------------------------
// Field helpers
// ---------------------------------------------------------------------------

fn non_empty(
    errors: &mut Vec<ValidationIssue>,
    code: &str,
    path: impl Into<String>,
    value: &str,
    field: &str,
) {
    if value.is_empty() {
        errors.push(ValidationIssue::new(
            code,
            path,
            format!("{field} must be a non-empty string"),
        ));
    }
}

fn check_uri(
    errors: &mut Vec<ValidationIssue>,
    code: &str,
    path: impl Into<String>,
    value: &str,
    field: &str,
) {
    if !is_uri(value) {
        errors.push(ValidationIssue::new(
            code,
            path,
            format!("{field} must be a URI (got `{value}`)"),
        ));
    }
}

fn check_rfc3339(
    errors: &mut Vec<ValidationIssue>,
    code: &str,
    path: impl Into<String>,
    value: &str,
    field: &str,
) {
    if !is_rfc3339(value) {
        errors.push(ValidationIssue::new(
            code,
            path,
            format!("{field} is not a valid RFC 3339 timestamp (got `{value}`)"),
        ));
    }
}

/// `scheme:rest` where scheme starts with a letter and continues with
/// `A-Za-z0-9+.-`, and `rest` is non-empty without whitespace (SPEC.md V-16).
pub fn is_uri(value: &str) -> bool {
    let Some((scheme, rest)) = value.split_once(':') else {
        return false;
    };
    let mut scheme_bytes = scheme.bytes();
    match scheme_bytes.next() {
        Some(first) if first.is_ascii_alphabetic() => {}
        _ => return false,
    }
    scheme_bytes.all(|b| b.is_ascii_alphanumeric() || b == b'+' || b == b'.' || b == b'-')
        && !rest.is_empty()
        && !rest.chars().any(char::is_whitespace)
}

pub fn is_lowercase_hex(value: &str) -> bool {
    value
        .bytes()
        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// RFC 3339 `date-time` with full calendar validation (SPEC.md V-14).
pub fn is_rfc3339(value: &str) -> bool {
    let b = value.as_bytes();
    if b.len() < 20 {
        return false;
    }
    let num = |start: usize, len: usize| -> Option<u32> {
        if start + len > b.len() {
            return None;
        }
        let mut v = 0u32;
        for byte in &b[start..start + len] {
            if !byte.is_ascii_digit() {
                return None;
            }
            v = v * 10 + u32::from(*byte - b'0');
        }
        Some(v)
    };
    let Some(year) = num(0, 4) else { return false };
    if b[4] != b'-' {
        return false;
    }
    let Some(month) = num(5, 2) else { return false };
    if !(1..=12).contains(&month) {
        return false;
    }
    if b[7] != b'-' {
        return false;
    }
    let Some(day) = num(8, 2) else { return false };
    if day < 1 || day > days_in_month(year, month) {
        return false;
    }
    if b[10] != b'T' && b[10] != b't' {
        return false;
    }
    let Some(hour) = num(11, 2) else { return false };
    if hour > 23 {
        return false;
    }
    if b[13] != b':' {
        return false;
    }
    let Some(minute) = num(14, 2) else {
        return false;
    };
    if minute > 59 {
        return false;
    }
    if b[16] != b':' {
        return false;
    }
    let Some(second) = num(17, 2) else {
        return false;
    };
    if second > 60 {
        // 60 allowed: leap second
        return false;
    }
    let mut i = 19;
    if b[19] == b'.' {
        i = 20;
        let start = i;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        if i == start {
            return false;
        }
    }
    match b.get(i) {
        Some(&b'Z') | Some(&b'z') => i + 1 == b.len(),
        Some(&b'+') | Some(&b'-') => {
            b.len() == i + 6
                && b.get(i + 3) == Some(&b':')
                && num(i + 1, 2).is_some_and(|h| h <= 23)
                && num(i + 4, 2).is_some_and(|m| m <= 59)
        }
        _ => false,
    }
}

fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn is_leap(year: u32) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}

// ---------------------------------------------------------------------------
// Report assembly
// ---------------------------------------------------------------------------

fn sort_issues(issues: &mut Vec<ValidationIssue>) {
    issues.sort_by(|a, b| {
        a.path
            .cmp(&b.path)
            .then(a.code.cmp(&b.code))
            .then(a.message.cmp(&b.message))
    });
    let mut seen: HashSet<(String, String)> = HashSet::new();
    issues.retain(|issue| seen.insert((issue.code.clone(), issue.path.clone())));
}

fn finish_report(
    mut errors: Vec<ValidationIssue>,
    mut warnings: Vec<ValidationIssue>,
    document_id: Option<String>,
    object_count: usize,
) -> ValidationReport {
    sort_issues(&mut errors);
    sort_issues(&mut warnings);
    ValidationReport {
        valid: errors.is_empty(),
        errors,
        warnings,
        document_id,
        object_count,
    }
}

/// Full validation of an already-parsed JSON value: structural subset, typed
/// parse, then the semantic layer. Never panics on malformed input.
pub fn validate_value(value: &Value) -> ValidationReport {
    let mut errors = schema_subset_issues(value);
    match serde_json::from_value::<SpatialDppDocument>(value.clone()) {
        Ok(doc) => {
            let (semantic_errors, warnings) = semantic_issues(&doc);
            errors.extend(semantic_errors);
            finish_report(
                errors,
                warnings,
                Some(doc.metadata.document_id.clone()),
                doc.objects.len(),
            )
        }
        Err(err) => {
            errors.push(ValidationIssue::new(
                "V-01",
                "",
                format!("document does not deserialize into the spatial-dpp data model: {err}"),
            ));
            let document_id = value
                .pointer("/metadata/document_id")
                .and_then(Value::as_str)
                .map(str::to_string);
            let object_count = value
                .pointer("/objects")
                .and_then(Value::as_array)
                .map_or(0, Vec::len);
            finish_report(errors, Vec::new(), document_id, object_count)
        }
    }
}

/// Semantic validation only, for an already-typed document (used by callers
/// that parse with [`crate::parse_document`] and by tests).
pub fn validate_document(doc: &SpatialDppDocument) -> ValidationReport {
    let (errors, warnings) = semantic_issues(doc);
    finish_report(
        errors,
        warnings,
        Some(doc.metadata.document_id.clone()),
        doc.objects.len(),
    )
}

/// Validate raw bytes in any supported format (auto-detected unless given).
/// Structural protobuf violations (unknown enums, missing `metadata`) are
/// returned as an invalid report, not as an error.
pub fn validate_bytes(data: &[u8], format: Option<Format>) -> Result<ValidationReport, ParseError> {
    match decode_to_value(data, format) {
        Ok((_, value)) => Ok(validate_value(&value)),
        Err(ParseError::InvalidModel(issues)) => Ok(finish_report(issues, Vec::new(), None, 0)),
        Err(err) => Err(err),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc3339_boundaries() {
        assert!(is_rfc3339("2026-01-01T00:00:00Z"));
        assert!(is_rfc3339("2026-12-31T23:59:60Z")); // leap second
        assert!(is_rfc3339("2026-06-01t12:00:00.123456+09:00"));
        assert!(is_rfc3339("2024-02-29T00:00:00Z")); // 2024 is a leap year
        assert!(!is_rfc3339("2026-13-01T00:00:00Z"));
        assert!(!is_rfc3339("2026-02-30T00:00:00Z"));
        assert!(!is_rfc3339("2026-02-29T00:00:00Z")); // 2026 is not
        assert!(!is_rfc3339("2026-01-01"));
        assert!(!is_rfc3339("2026-01-01T00:00:00")); // missing offset
        assert!(!is_rfc3339("2026-01-01 00:00:00Z")); // space instead of T
        assert!(!is_rfc3339("2026-01-01T24:00:00Z"));
        assert!(!is_rfc3339(""));
    }

    #[test]
    fn uri_shapes() {
        assert!(is_uri("urn:product:ev-2026"));
        assert!(is_uri("URN:PRODUCT:EV"));
        assert!(is_uri("https://example.com/ev-pack.json"));
        assert!(is_uri("file:///tmp/x.glb"));
        assert!(!is_uri("notauri"));
        assert!(!is_uri("https://example.com/a b")); // whitespace not allowed
        assert!(!is_uri(":nocolon"));
        assert!(!is_uri("1urn:x")); // scheme must start with a letter
        assert!(!is_uri("urn:")); // empty rest
    }

    #[test]
    fn embedded_schema_parses() {
        assert!(serde_json::from_str::<Value>(EMBEDDED_SCHEMA).is_ok());
    }
}
