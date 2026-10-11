//! Boundary-value tests for every semantic rule (SPEC.md Section 6).
//! Documents are built programmatically so NaN/±∞ inputs can be tested
//! even though JSON cannot represent them.

use sdpp_core::model::*;
use sdpp_core::validate::validate_document;

fn object(id: &str) -> SpatialObject {
    SpatialObject {
        id: id.to_string(),
        label: format!("Object {id}"),
        geometry: GeometryRef {
            format: GeometryFormat::Gltf,
            uri: "urn:test:mesh".to_string(),
            hash: Some(ContentHash {
                algorithm: HashAlgorithm::Sha256,
                value: "a".repeat(64),
            }),
            locator: None,
            scale_hint_m: None,
        },
        physical: PhysicalProperties {
            mass_kg: 1.0,
            center_of_mass: vec![0.5, 0.5, 0.5],
            density_kg_m3: None,
            friction: FrictionModel {
                static_coefficient: 0.5,
                kinetic: None,
            },
            stiffness: Stiffness {
                youngs_modulus_pa: 1_000_000.0,
                shear_modulus_pa: None,
                poisson_ratio: None,
            },
            joints: vec![],
        },
        compliance: ComplianceRecord {
            bill_of_materials: vec![BomEntry {
                part_id: "p1".to_string(),
                label: "Part".to_string(),
                quantity: 1,
                material_id: Some("m1".to_string()),
                mass_kg: None,
            }],
            material_composition: vec![
                MaterialFraction {
                    material_id: "m1".to_string(),
                    label: "M1".to_string(),
                    fraction: 0.6,
                },
                MaterialFraction {
                    material_id: "m2".to_string(),
                    label: "M2".to_string(),
                    fraction: 0.4,
                },
            ],
            recyclability_rate: 0.5,
            dpp: DppRecord {
                passport_id: "urn:dpp:x".to_string(),
                sbom_ref: None,
                hbom_ref: None,
                repairability_score: None,
            },
            disassembly: None,
            cra: CraRecord {
                conformance: CraConformance::Conformant,
                support_period_until: None,
                vulnerability_disclosure_uri: None,
                logs: vec![],
            },
        },
    }
}

fn base_doc() -> SpatialDppDocument {
    SpatialDppDocument {
        metadata: DocumentMetadata {
            spec_version: "0.1.0".to_string(),
            document_id: "urn:test:doc".to_string(),
            name: "Base".to_string(),
            created: "2026-01-01T00:00:00Z".to_string(),
            author: None,
            language: None,
        },
        objects: vec![object("a"), object("b")],
    }
}

fn with_joint(mut doc: SpatialDppDocument) -> SpatialDppDocument {
    doc.objects[0].physical.joints = vec![KinematicJoint {
        id: "j1".to_string(),
        joint_type: JointType::Revolute,
        axis: Some(vec![1.0, 0.0, 0.0]),
        limits: Some(JointLimits {
            lower_deg: Some(-10.0),
            upper_deg: Some(10.0),
            lower_m: None,
            upper_m: None,
        }),
        parent: "a".to_string(),
        child: "b".to_string(),
    }];
    doc
}

fn mutate(
    mut doc: SpatialDppDocument,
    f: impl FnOnce(&mut SpatialDppDocument),
) -> SpatialDppDocument {
    f(&mut doc);
    doc
}

fn error_codes(doc: &SpatialDppDocument) -> Vec<String> {
    let mut codes: Vec<String> = validate_document(doc)
        .errors
        .iter()
        .map(|e| e.code.clone())
        .collect();
    codes.sort();
    codes.dedup();
    codes
}

fn warning_codes(doc: &SpatialDppDocument) -> Vec<String> {
    let mut codes: Vec<String> = validate_document(doc)
        .warnings
        .iter()
        .map(|e| e.code.clone())
        .collect();
    codes.sort();
    codes.dedup();
    codes
}

fn assert_codes(doc: &SpatialDppDocument, expected: &[&str]) {
    let expected: Vec<String> = expected.iter().map(|s| s.to_string()).collect();
    assert_eq!(error_codes(doc), expected);
}

#[test]
fn base_document_is_valid() {
    assert_eq!(error_codes(&base_doc()), Vec::<String>::new());
    // base doc has no disassembly plans → one W-03 per object
    assert_eq!(warning_codes(&base_doc()), vec!["W-03"]);
}

#[test]
fn mass_boundaries() {
    for bad in [0.0, -1e-9, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let doc = mutate(base_doc(), |d| d.objects[0].physical.mass_kg = bad);
        assert_codes(&doc, &["V-02"]);
    }
    let ok = mutate(base_doc(), |d| {
        d.objects[0].physical.mass_kg = f64::MIN_POSITIVE
    });
    assert_eq!(error_codes(&ok), Vec::<String>::new());
}

#[test]
fn friction_boundaries() {
    let negative = mutate(base_doc(), |d| {
        d.objects[0].physical.friction.static_coefficient = -0.001
    });
    assert_codes(&negative, &["V-03"]);
    let nan = mutate(base_doc(), |d| {
        d.objects[0].physical.friction.static_coefficient = f64::NAN
    });
    assert_codes(&nan, &["V-03"]);
    let kinetic = mutate(base_doc(), |d| {
        d.objects[0].physical.friction.kinetic = Some(-0.5);
    });
    assert_codes(&kinetic, &["V-03"]);
    let zero = mutate(base_doc(), |d| {
        d.objects[0].physical.friction.static_coefficient = 0.0;
    });
    assert_eq!(error_codes(&zero), Vec::<String>::new());
}

#[test]
fn stiffness_and_density_boundaries() {
    for (label, f) in [
        (
            "youngs zero",
            Box::new(|d: &mut SpatialDppDocument| {
                d.objects[0].physical.stiffness.youngs_modulus_pa = 0.0;
            }) as Box<dyn FnOnce(&mut SpatialDppDocument)>,
        ),
        (
            "shear negative",
            Box::new(|d: &mut SpatialDppDocument| {
                d.objects[0].physical.stiffness.shear_modulus_pa = Some(-1.0);
            }),
        ),
        (
            "poisson at 0.5",
            Box::new(|d: &mut SpatialDppDocument| {
                d.objects[0].physical.stiffness.poisson_ratio = Some(0.5);
            }),
        ),
        (
            "poisson negative",
            Box::new(|d: &mut SpatialDppDocument| {
                d.objects[0].physical.stiffness.poisson_ratio = Some(-0.1);
            }),
        ),
        (
            "density zero",
            Box::new(|d: &mut SpatialDppDocument| {
                d.objects[0].physical.density_kg_m3 = Some(0.0);
            }),
        ),
    ] {
        let doc = mutate(base_doc(), f);
        assert_eq!(error_codes(&doc), vec!["V-04"], "case {label}");
    }
    let ok = mutate(base_doc(), |d| {
        d.objects[0].physical.stiffness.poisson_ratio = Some(0.4999);
    });
    assert_eq!(error_codes(&ok), Vec::<String>::new());
}

#[test]
fn center_of_mass_boundaries() {
    let zero = mutate(base_doc(), |d| {
        d.objects[0].physical.center_of_mass = vec![0.0, 0.0, 0.0]
    });
    assert_eq!(error_codes(&zero), Vec::<String>::new());
    let one = mutate(base_doc(), |d| {
        d.objects[0].physical.center_of_mass = vec![1.0, 1.0, 1.0]
    });
    assert_eq!(error_codes(&one), Vec::<String>::new());
    for bad in [
        vec![1.0000001, 0.0, 0.0],
        vec![-0.001, 0.0, 0.0],
        vec![f64::NAN, 0.0, 0.0],
        vec![0.5, 0.5],
        vec![0.5, 0.5, 0.5, 0.5],
    ] {
        let doc = mutate(base_doc(), |d| {
            d.objects[0].physical.center_of_mass = bad.clone()
        });
        assert_eq!(error_codes(&doc), vec!["V-05"], "com {bad:?}");
    }
}

#[test]
fn joint_axis_boundaries() {
    assert_eq!(error_codes(&with_joint(base_doc())), Vec::<String>::new());
    let nearly_unit = mutate(with_joint(base_doc()), |d| {
        d.objects[0].physical.joints[0].axis = Some(vec![0.9999999, 0.0, 0.0]);
    });
    assert_eq!(error_codes(&nearly_unit), Vec::<String>::new());
    for bad in [
        Some(vec![0.9, 0.0, 0.0]),
        Some(vec![f64::NAN, 0.0, 0.0]),
        Some(vec![1.0, 0.0]),
        None,
    ] {
        let doc = mutate(with_joint(base_doc()), |d| {
            d.objects[0].physical.joints[0].axis = bad.clone()
        });
        assert_eq!(error_codes(&doc), vec!["V-06"], "axis {bad:?}");
    }
}

#[test]
fn joint_type_rules() {
    // prismatic requires limits
    let no_limits = mutate(with_joint(base_doc()), |d| {
        d.objects[0].physical.joints[0].joint_type = JointType::Prismatic;
        d.objects[0].physical.joints[0].limits = None;
    });
    assert_codes(&no_limits, &["V-06"]);
    // prismatic with degree keys is wrong
    let deg_keys = mutate(with_joint(base_doc()), |d| {
        d.objects[0].physical.joints[0].joint_type = JointType::Prismatic;
        d.objects[0].physical.joints[0].limits = Some(JointLimits {
            lower_deg: Some(-10.0),
            upper_deg: Some(10.0),
            lower_m: None,
            upper_m: None,
        });
    });
    assert_codes(&deg_keys, &["V-06"]);
    // revolute limits must be ordered
    let reversed = mutate(with_joint(base_doc()), |d| {
        d.objects[0].physical.joints[0].limits = Some(JointLimits {
            lower_deg: Some(10.0),
            upper_deg: Some(-10.0),
            lower_m: None,
            upper_m: None,
        });
    });
    assert_codes(&reversed, &["V-06"]);
    // equal limits are fine
    let equal = mutate(with_joint(base_doc()), |d| {
        d.objects[0].physical.joints[0].limits = Some(JointLimits {
            lower_deg: Some(-10.0),
            upper_deg: Some(-10.0),
            lower_m: None,
            upper_m: None,
        });
    });
    assert_eq!(error_codes(&equal), Vec::<String>::new());
    // revolute with only lower_deg is incomplete
    let partial = mutate(with_joint(base_doc()), |d| {
        d.objects[0].physical.joints[0].limits = Some(JointLimits {
            lower_deg: Some(-10.0),
            upper_deg: None,
            lower_m: None,
            upper_m: None,
        });
    });
    assert_codes(&partial, &["V-06"]);
    // fixed joints need no axis
    let fixed = mutate(with_joint(base_doc()), |d| {
        d.objects[0].physical.joints[0].joint_type = JointType::Fixed;
        d.objects[0].physical.joints[0].axis = None;
        d.objects[0].physical.joints[0].limits = None;
    });
    assert_eq!(error_codes(&fixed), Vec::<String>::new());
}

#[test]
fn joint_reference_rules() {
    let bad_parent = mutate(with_joint(base_doc()), |d| {
        d.objects[0].physical.joints[0].parent = "ghost".to_string();
    });
    assert_codes(&bad_parent, &["V-08"]);
    let bad_child = mutate(with_joint(base_doc()), |d| {
        d.objects[0].physical.joints[0].child = "ghost".to_string();
    });
    assert_codes(&bad_child, &["V-08"]);
    let self_loop = mutate(with_joint(base_doc()), |d| {
        d.objects[0].physical.joints[0].child = "a".to_string();
    });
    assert_codes(&self_loop, &["V-08"]);
}

#[test]
fn fraction_rules() {
    let undersum = mutate(base_doc(), |d| {
        d.objects[0].compliance.material_composition[0].fraction = 0.5;
    });
    assert_codes(&undersum, &["V-09"]);
    let oversum = mutate(base_doc(), |d| {
        d.objects[0].compliance.material_composition[0].fraction = 0.7;
    });
    assert_codes(&oversum, &["V-09"]);
    let within_tolerance = mutate(base_doc(), |d| {
        d.objects[0].compliance.material_composition[0].fraction = 0.5995;
    });
    assert_eq!(error_codes(&within_tolerance), Vec::<String>::new());
    let zero_fraction = mutate(base_doc(), |d| {
        d.objects[0].compliance.material_composition[0].fraction = 0.0;
    });
    assert_codes(&zero_fraction, &["V-09"]);
    let over_one = mutate(base_doc(), |d| {
        d.objects[0].compliance.material_composition[0].fraction = 1.01;
    });
    assert_codes(&over_one, &["V-09"]);
    let single_whole = mutate(base_doc(), |d| {
        d.objects[0].compliance.material_composition = vec![MaterialFraction {
            material_id: "m1".to_string(),
            label: "M1".to_string(),
            fraction: 1.0,
        }];
    });
    assert_eq!(error_codes(&single_whole), Vec::<String>::new());
}

#[test]
fn bom_material_reference_rules() {
    let ghost = mutate(base_doc(), |d| {
        d.objects[0].compliance.bill_of_materials[0].material_id = Some("ghost".to_string());
    });
    assert_codes(&ghost, &["V-10"]);
    let none = mutate(base_doc(), |d| {
        d.objects[0].compliance.bill_of_materials[0].material_id = None;
    });
    assert_eq!(error_codes(&none), Vec::<String>::new());
}

#[test]
fn rate_rules() {
    let over = mutate(base_doc(), |d| {
        d.objects[0].compliance.recyclability_rate = 1.5;
    });
    assert_codes(&over, &["V-11"]);
    let negative = mutate(base_doc(), |d| {
        d.objects[0].compliance.recyclability_rate = -0.1;
    });
    assert_codes(&negative, &["V-11"]);
    let repairability = mutate(base_doc(), |d| {
        d.objects[0].compliance.dpp.repairability_score = Some(1.5);
    });
    assert_codes(&repairability, &["V-11"]);
    let bounds_ok = mutate(base_doc(), |d| {
        d.objects[0].compliance.recyclability_rate = 1.0;
        d.objects[0].compliance.dpp.repairability_score = Some(0.0);
    });
    assert_eq!(error_codes(&bounds_ok), Vec::<String>::new());
}

#[test]
fn disassembly_rules() {
    let plan = |steps: Vec<DisassemblyStep>, doc: &mut SpatialDppDocument, index: usize| {
        doc.objects[index].compliance.disassembly = Some(DisassemblyPlan {
            plan_ref: None,
            steps,
        });
    };
    let mut ok = base_doc();
    plan(
        vec![
            DisassemblyStep {
                order: 1,
                action: DisassemblyAction::Unfasten,
                target: "p1".to_string(),
                tool: None,
                hazard: None,
                note: None,
            },
            DisassemblyStep {
                order: 2,
                action: DisassemblyAction::Lift,
                target: "a".to_string(),
                tool: None,
                hazard: None,
                note: None,
            },
        ],
        &mut ok,
        0,
    );
    assert_eq!(error_codes(&ok), Vec::<String>::new());

    let mut skipped_order = base_doc();
    plan(
        vec![
            DisassemblyStep {
                order: 1,
                action: DisassemblyAction::Unfasten,
                target: "p1".to_string(),
                tool: None,
                hazard: None,
                note: None,
            },
            DisassemblyStep {
                order: 3,
                action: DisassemblyAction::Lift,
                target: "p1".to_string(),
                tool: None,
                hazard: None,
                note: None,
            },
        ],
        &mut skipped_order,
        0,
    );
    assert_codes(&skipped_order, &["V-12"]);

    let mut starts_at_two = base_doc();
    plan(
        vec![DisassemblyStep {
            order: 2,
            action: DisassemblyAction::Unfasten,
            target: "p1".to_string(),
            tool: None,
            hazard: None,
            note: None,
        }],
        &mut starts_at_two,
        0,
    );
    assert_codes(&starts_at_two, &["V-12"]);

    let mut ghost_target = base_doc();
    plan(
        vec![DisassemblyStep {
            order: 1,
            action: DisassemblyAction::Cut,
            target: "ghost".to_string(),
            tool: None,
            hazard: None,
            note: None,
        }],
        &mut ghost_target,
        0,
    );
    assert_codes(&ghost_target, &["V-12"]);
}

#[test]
fn hash_rules() {
    let short = mutate(base_doc(), |d| {
        d.objects[0].geometry.hash = Some(ContentHash {
            algorithm: HashAlgorithm::Sha256,
            value: "a".repeat(63),
        });
    });
    assert_codes(&short, &["V-13"]);
    let uppercase = mutate(base_doc(), |d| {
        d.objects[0].geometry.hash = Some(ContentHash {
            algorithm: HashAlgorithm::Sha256,
            value: "A".repeat(64),
        });
    });
    assert_codes(&uppercase, &["V-13"]);
    let sha512_short = mutate(base_doc(), |d| {
        d.objects[0].geometry.hash = Some(ContentHash {
            algorithm: HashAlgorithm::Sha512,
            value: "a".repeat(64),
        });
    });
    assert_codes(&sha512_short, &["V-13"]);
    let sha512_ok = mutate(base_doc(), |d| {
        d.objects[0].geometry.hash = Some(ContentHash {
            algorithm: HashAlgorithm::Sha512,
            value: "a".repeat(128),
        });
    });
    assert_eq!(error_codes(&sha512_ok), Vec::<String>::new());
}

#[test]
fn timestamp_rules() {
    let bad_support = mutate(base_doc(), |d| {
        d.objects[0].compliance.cra.support_period_until = Some("2026-02-30T00:00:00Z".to_string());
    });
    assert_codes(&bad_support, &["V-14"]);
    let bad_log = mutate(base_doc(), |d| {
        d.objects[0].compliance.cra.logs = vec![CraLogEntry {
            timestamp: "2026-01-01 00:00:00".to_string(),
            event: "e".to_string(),
            detail: None,
        }];
    });
    assert_codes(&bad_log, &["V-14"]);
    let leap_ok = mutate(base_doc(), |d| {
        d.objects[0].compliance.cra.support_period_until = Some("2024-02-29T12:00:00Z".to_string());
    });
    assert_eq!(error_codes(&leap_ok), Vec::<String>::new());
}

#[test]
fn quantity_and_scale_rules() {
    let zero_quantity = mutate(base_doc(), |d| {
        d.objects[0].compliance.bill_of_materials[0].quantity = 0;
    });
    assert_codes(&zero_quantity, &["V-15"]);
    let short_scale = mutate(base_doc(), |d| {
        d.objects[0].geometry.scale_hint_m = Some(vec![0.1, 0.2]);
    });
    assert_codes(&short_scale, &["V-15"]);
    let negative_scale = mutate(base_doc(), |d| {
        d.objects[0].geometry.scale_hint_m = Some(vec![0.1, 0.2, -0.3]);
    });
    assert_codes(&negative_scale, &["V-15"]);
    let ok_scale = mutate(base_doc(), |d| {
        d.objects[0].geometry.scale_hint_m = Some(vec![0.1, 0.2, 0.3]);
    });
    assert_eq!(error_codes(&ok_scale), Vec::<String>::new());
}

#[test]
fn uri_rules() {
    let bad_document_id = mutate(base_doc(), |d| d.metadata.document_id = "nope".to_string());
    assert_codes(&bad_document_id, &["V-16"]);
    let bad_sbom = mutate(base_doc(), |d| {
        d.objects[0].compliance.dpp.sbom_ref = Some("not-a-uri".to_string());
    });
    assert_codes(&bad_sbom, &["V-16"]);
    let bad_plan_ref = mutate(base_doc(), |d| {
        d.objects[0].compliance.disassembly = Some(DisassemblyPlan {
            plan_ref: Some("no scheme here".to_string()),
            steps: vec![],
        });
    });
    assert_codes(&bad_plan_ref, &["V-16"]);
    let ok_uri = mutate(base_doc(), |d| {
        d.objects[0].compliance.dpp.hbom_ref = Some("file:///hbom/x".to_string());
    });
    assert_eq!(error_codes(&ok_uri), Vec::<String>::new());
}

#[test]
fn non_empty_string_rules() {
    for (label, f) in [
        (
            "name",
            Box::new(|d: &mut SpatialDppDocument| d.metadata.name = String::new())
                as Box<dyn FnOnce(&mut SpatialDppDocument)>,
        ),
        (
            "object label",
            Box::new(|d: &mut SpatialDppDocument| {
                d.objects[0].label = String::new();
            }),
        ),
        (
            "part label",
            Box::new(|d: &mut SpatialDppDocument| {
                d.objects[0].compliance.bill_of_materials[0].label = String::new();
            }),
        ),
        (
            "passport",
            Box::new(|d: &mut SpatialDppDocument| {
                d.objects[0].compliance.dpp.passport_id = String::new();
            }),
        ),
    ] {
        let doc = mutate(base_doc(), f);
        assert_eq!(error_codes(&doc), vec!["V-17"], "case {label}");
    }
}

#[test]
fn uniqueness_rules() {
    let duplicate_object = mutate(base_doc(), |d| d.objects[1].id = "a".to_string());
    assert_codes(&duplicate_object, &["V-07"]);
    let duplicate_part = mutate(base_doc(), |d| {
        d.objects[0].compliance.bill_of_materials.push(BomEntry {
            part_id: "p1".to_string(),
            label: "Part again".to_string(),
            quantity: 1,
            material_id: None,
            mass_kg: None,
        });
    });
    assert_codes(&duplicate_part, &["V-07"]);
    let duplicate_material = mutate(base_doc(), |d| {
        d.objects[0].compliance.material_composition[1].material_id = "m1".to_string();
    });
    assert_codes(&duplicate_material, &["V-07"]);
    let duplicate_joint = mutate(with_joint(base_doc()), |d| {
        d.objects[0].physical.joints.push(KinematicJoint {
            id: "j1".to_string(),
            joint_type: JointType::Revolute,
            axis: Some(vec![1.0, 0.0, 0.0]),
            limits: Some(JointLimits {
                lower_deg: Some(-10.0),
                upper_deg: Some(10.0),
                lower_m: None,
                upper_m: None,
            }),
            parent: "a".to_string(),
            child: "b".to_string(),
        });
    });
    assert_codes(&duplicate_joint, &["V-07"]);
}

#[test]
fn structural_twins() {
    let empty_objects = mutate(base_doc(), |d| d.objects.clear());
    assert_codes(&empty_objects, &["V-01"]);
    let empty_bom = mutate(base_doc(), |d| {
        d.objects[0].compliance.bill_of_materials.clear();
    });
    assert_codes(&empty_bom, &["V-01"]);
    let empty_composition = mutate(base_doc(), |d| {
        d.objects[0].compliance.bill_of_materials[0].material_id = None;
        d.objects[0].compliance.material_composition.clear();
    });
    assert_codes(&empty_composition, &["V-01"]);
    let wrong_version = mutate(base_doc(), |d| {
        d.metadata.spec_version = "0.2.0".to_string()
    });
    assert_codes(&wrong_version, &["V-01"]);
}

#[test]
fn warning_rules() {
    let no_hash = mutate(base_doc(), |d| d.objects[0].geometry.hash = None);
    assert!(warning_codes(&no_hash).contains(&"W-01".to_string()));
    let unknown_cra = mutate(base_doc(), |d| {
        d.objects[0].compliance.cra.conformance = CraConformance::Unknown;
    });
    assert!(warning_codes(&unknown_cra).contains(&"W-02".to_string()));
    let no_disassembly = base_doc();
    assert!(warning_codes(&no_disassembly).contains(&"W-03".to_string()));
    let all_warnings = mutate(
        mutate(base_doc(), |d| {
            d.objects[0].geometry.hash = None;
            d.objects[0].compliance.cra.conformance = CraConformance::Unknown;
        }),
        |d| d.objects[1].geometry.hash = None,
    );
    assert_eq!(warning_codes(&all_warnings), vec!["W-01", "W-02", "W-03"]);
}
