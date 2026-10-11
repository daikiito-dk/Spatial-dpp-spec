//! Human-readable summaries (the `spatial-dpp inspect` output).
//! Byte-deterministic for equal documents (golden-tested).

use crate::model::{CraConformance, SpatialDppDocument};

/// Render a document as a human-readable summary.
pub fn summary(doc: &SpatialDppDocument) -> String {
    let mut out = String::new();
    let metadata = &doc.metadata;
    out.push_str(&format!("Document:   {}\n", metadata.name));
    out.push_str(&format!("ID:         {}\n", metadata.document_id));
    out.push_str(&format!(
        "Spec:       spatial-dpp {}\n",
        metadata.spec_version
    ));
    out.push_str(&format!("Created:    {}\n", metadata.created));
    out.push_str(&format!("Objects:    {}\n", doc.objects.len()));

    for object in &doc.objects {
        out.push('\n');
        out.push_str(&format!("[{}] {}\n", object.id, object.label));
        let geometry = &object.geometry;
        out.push_str(&format!(
            "  Geometry:    {} {}\n",
            geometry.format.as_str(),
            geometry.uri
        ));
        match &geometry.hash {
            Some(hash) => out.push_str(&format!(
                "  Integrity:   {}:{}..\n",
                hash.algorithm.as_str(),
                &hash.value[..8.min(hash.value.len())]
            )),
            None => out.push_str("  Integrity:   (no hash)\n"),
        }
        if let Some(locator) = &geometry.locator {
            out.push_str(&format!("  Locator:     {locator}\n"));
        }
        let physical = &object.physical;
        out.push_str(&format!("  Mass:        {} kg\n", physical.mass_kg));
        out.push_str(&format!(
            "  COM:         {} bbox-normalized\n",
            fmt_com(&physical.center_of_mass)
        ));
        match physical.friction.kinetic {
            Some(kinetic) => out.push_str(&format!(
                "  Friction:    static {}, kinetic {}\n",
                physical.friction.static_coefficient, kinetic
            )),
            None => out.push_str(&format!(
                "  Friction:    static {}\n",
                physical.friction.static_coefficient
            )),
        }
        let mut stiffness = format!(
            "  Stiffness:   E = {} Pa",
            physical.stiffness.youngs_modulus_pa
        );
        if let Some(shear) = physical.stiffness.shear_modulus_pa {
            stiffness.push_str(&format!(", G = {shear} Pa"));
        }
        if let Some(poisson) = physical.stiffness.poisson_ratio {
            stiffness.push_str(&format!(", poisson {poisson}"));
        }
        out.push_str(&stiffness);
        out.push('\n');
        if let Some(density) = physical.density_kg_m3 {
            out.push_str(&format!("  Density:     {density} kg/m3\n"));
        }
        if !physical.joints.is_empty() {
            out.push_str(&format!("  Joints:      {}\n", physical.joints.len()));
            for joint in &physical.joints {
                let mut line = format!(
                    "    {}: {}, parent={}, child={}",
                    joint.id,
                    joint.joint_type.as_str(),
                    joint.parent,
                    joint.child
                );
                if let Some(axis) = &joint.axis {
                    line.push_str(&format!(", axis={}", fmt_vec3(axis)));
                }
                if let Some(limits) = &joint.limits {
                    if let (Some(lo), Some(hi)) = (limits.lower_deg, limits.upper_deg) {
                        line.push_str(&format!(", limits=[{lo:.3} deg, {hi:.3} deg]"));
                    }
                    if let (Some(lo), Some(hi)) = (limits.lower_m, limits.upper_m) {
                        line.push_str(&format!(", limits=[{lo:.3} m, {hi:.3} m]"));
                    }
                }
                out.push_str(&line);
                out.push('\n');
            }
        }
        let compliance = &object.compliance;
        out.push_str(&format!(
            "  BOM:         {} parts\n",
            compliance.bill_of_materials.len()
        ));
        let materials: Vec<String> = compliance
            .material_composition
            .iter()
            .map(|f| format!("{} {}", f.material_id, f.fraction))
            .collect();
        out.push_str(&format!("  Materials:   {}\n", materials.join(", ")));
        out.push_str(&format!(
            "  Recyclable:  {:.1}%\n",
            compliance.recyclability_rate * 100.0
        ));
        out.push_str(&format!("  Passport:    {}\n", compliance.dpp.passport_id));
        if let Some(sbom) = &compliance.dpp.sbom_ref {
            out.push_str(&format!("  SBOM:        {sbom}\n"));
        }
        if let Some(hbom) = &compliance.dpp.hbom_ref {
            out.push_str(&format!("  HBOM:        {hbom}\n"));
        }
        let cra_line = match compliance.cra.conformance {
            CraConformance::Conformant => "conformant".to_string(),
            CraConformance::Partial => "partial".to_string(),
            CraConformance::Unknown => "unknown".to_string(),
        };
        match &compliance.cra.support_period_until {
            Some(until) => out.push_str(&format!(
                "  CRA:         {cra_line} (support until {until})\n"
            )),
            None => out.push_str(&format!("  CRA:         {cra_line}\n")),
        }
        match &compliance.disassembly {
            Some(plan) if !plan.steps.is_empty() => {
                let steps: Vec<String> = plan
                    .steps
                    .iter()
                    .map(|s| format!("{} {}({})", s.order, s.action.as_str(), s.target))
                    .collect();
                out.push_str(&format!(
                    "  Disassembly: {} steps: {}\n",
                    plan.steps.len(),
                    steps.join("; ")
                ));
            }
            Some(plan) => out.push_str(&format!(
                "  Disassembly: 0 steps (plan_ref: {})\n",
                plan.plan_ref.as_deref().unwrap_or("-")
            )),
            None => out.push_str("  Disassembly: (none)\n"),
        }
    }
    out
}

fn fmt_com(com: &[f64]) -> String {
    if com.len() == 3 {
        format!("({:.3}, {:.3}, {:.3})", com[0], com[1], com[2])
    } else {
        format!(
            "({})",
            com.iter()
                .map(|v| format!("{v}"))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

fn fmt_vec3(values: &[f64]) -> String {
    if values.len() == 3 {
        format!("({:.3}, {:.3}, {:.3})", values[0], values[1], values[2])
    } else {
        format!(
            "({})",
            values
                .iter()
                .map(|v| format!("{v}"))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}
