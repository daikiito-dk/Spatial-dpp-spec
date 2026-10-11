# Spatial DPP Specification

**Version:** 0.1.0 (draft)
**Status:** Single source of truth (SSOT) for the `spatial-dpp` data model, validation rules, and serialization formats.
**Canonical identifiers:**

- JSON Schema: `https://spatial-dpp.org/schemas/v0.1.0/spatial-dpp.schema.json` (local copy: `schemas/v0.1.0/spatial-dpp.schema.json`)
- JSON-LD context: `https://spatial-dpp.org/contexts/v0.1.0` (local copy: `schemas/v0.1.0/context.jsonld`)
- Protobuf schema: `schemas/v0.1.0/spatial_dpp.proto`
- glTF extension: `EXT_spatial_dpp` (see `docs/gltf-extension/EXT_spatial_dpp.md`)

---

## 1. Purpose and scope

`spatial-dpp` is a machine-readable, open data structure that binds, for one physical object:

1. **Geometry** — a non-destructive reference (URI + content hash) to an existing 3D/CAD asset (glTF / USD / STEP / OBJ / STL).
2. **Physical attributes** — mass, center of mass, friction, stiffness, and kinematic joints, in the units a physics engine consumes directly.
3. **Compliance & DPP attributes** — bill of materials, material composition, recyclability, disassembly steps, EU Digital Product Passport references (SBOM/HBOM), and Cyber Resilience Act (CRA) conformance records.

The three layers live in one document so that a robot, simulator, or auditor can answer, from a single parse: *"What is this object, how does it behave physically, and how do I take it apart safely and lawfully?"*

Out of scope for v0.1.0: geometry data itself, binary glTF (`.glb`) containers, cryptographic signing of passports, and preservation of vendor extension keys (`x-*`) through parse/serialize round-trips (planned for v0.2).

## 2. Terminology

| Term | Meaning |
|---|---|
| **DPP** | Digital Product Passport (EU ESPR / battery regulation) |
| **BOM** | Bill of materials |
| **SBOM / HBOM** | Software / Hardware Bill of Materials |
| **CRA** | EU Cyber Resilience Act |
| **OD-BOM** | Open Disassembly & Reverse BOM, the disassembly-planning format of the DeCAD project (`https://github.com/daikiito-dk/DeCAD`) |
| **Document** | One serialized `SpatialDppDocument` |
| **Object** | One addressed physical thing with its three layers |

## 3. Data model (normative)

Field names below are the canonical JSON names; all other serializations map onto them (Section 7). `?` marks optional fields. Arrays have fixed lengths where noted.

### 3.1 `SpatialDppDocument`

```
SpatialDppDocument
├─ metadata: DocumentMetadata          (required)
└─ objects: SpatialObject[]            (required, minItems 1)
```

### 3.2 `DocumentMetadata`

| Field | Type | Required | Constraints |
|---|---|---|---|
| `spec_version` | string | yes | `"0.1.0"` |
| `document_id` | string | yes | URI (`scheme:rest`), unique per issuer |
| `name` | string | yes | non-empty |
| `created` | string | yes | RFC 3339 timestamp |
| `author` | string | no | — |
| `language` | string | no | BCP 47 tag recommended |

### 3.3 `SpatialObject`

| Field | Type | Required |
|---|---|---|
| `id` | string | yes (unique in document) |
| `label` | string | yes |
| `geometry` | `GeometryRef` | yes |
| `physical` | `PhysicalProperties` | yes |
| `compliance` | `ComplianceRecord` | yes |

### 3.4 `GeometryRef` — non-destructive geometry reference

| Field | Type | Required | Constraints |
|---|---|---|---|
| `format` | enum: `gltf`, `usda`, `usdc`, `step`, `obj`, `stl` | yes | — |
| `uri` | string | yes | URI; must not be modified by writers of this spec |
| `hash` | `ContentHash?` | no (warned, W-01) | integrity of the referenced asset |
| `locator` | string | no | sub-address inside the asset, e.g. `#/nodes/3`, `/RootPack/Cells`, `#41=PRODUCT` |
| `scale_hint_m` | number[3] | no | authored bounding-box extents in meters, all > 0 |

`ContentHash = { algorithm: enum sha256|sha512, value: lowercase hex (64 / 128 chars) }`

### 3.5 `PhysicalProperties`

| Field | Type | Required | Constraints |
|---|---|---|---|
| `mass_kg` | number | yes | finite, > 0 |
| `center_of_mass` | number[3] | yes | each coordinate in `[0,1]` (see 4.2) |
| `density_kg_m3` | number? | no | > 0 |
| `friction` | `FrictionModel` | yes | — |
| `stiffness` | `Stiffness` | yes | — |
| `joints` | `KinematicJoint[]?` | no | — |

`FrictionModel = { static: number (≥ 0), kinetic?: number (≥ 0) }`
`Stiffness = { youngs_modulus_pa: number (> 0), shear_modulus_pa?: number (> 0), poisson_ratio?: number ([0, 0.5)) }`

### 3.6 `KinematicJoint`

| Field | Type | Required | Constraints |
|---|---|---|---|
| `id` | string | yes | unique within the object |
| `type` | enum: `revolute`, `prismatic`, `fixed`, `spherical` | yes | — |
| `axis` | number[3]? | yes unless `type = fixed` | unit vector, tolerance `1e-6` |
| `limits` | `JointLimits?` | no (required for `prismatic`) | correct keys per type |
| `parent` | string | yes | object `id` in the same document |
| `child` | string | yes | object `id` in the same document, ≠ `parent` |

`JointLimits` uses degree keys (`lower_deg`, `upper_deg`) for `revolute`/`spherical` and meter keys (`lower_m`, `upper_m`) for `prismatic`. In both cases `lower ≤ upper`.

### 3.7 `ComplianceRecord`

| Field | Type | Required |
|---|---|---|
| `bill_of_materials` | `BomEntry[]` | yes (minItems 1) |
| `material_composition` | `MaterialFraction[]` | yes (minItems 1) |
| `recyclability_rate` | number `[0,1]` | yes |
| `dpp` | `DppRecord` | yes |
| `disassembly` | `DisassemblyPlan?` | no (warned, W-03) |
| `cra` | `CraRecord` | yes |

```
BomEntry           = { part_id, label, quantity (uint ≥ 1), material_id?, mass_kg? (> 0) }
MaterialFraction   = { material_id, label, fraction ∈ (0, 1] }
DppRecord          = { passport_id: string (required),
                       sbom_ref?: URI, hbom_ref?: URI,
                       repairability_score?: number [0,1] }
DisassemblyPlan    = { plan_ref?: URI, steps?: DisassemblyStep[] }
DisassemblyStep    = { order: uint (strictly increasing from 1), action, target,
                       tool?, hazard?, note? }
action ∈ { unfasten, cut, lift, release, drain, deactivate, other }
hazard ∈ { none, thermal_runaway, chemical, asbestos, electrical, sharp, crush }
CraRecord          = { conformance: conformant | partial | unknown,
                       support_period_until?: RFC 3339,
                       vulnerability_disclosure_uri?: URI,
                       logs?: [ { timestamp: RFC 3339, event: string, detail? } ] }
```

## 4. Units and coordinate conventions

### 4.1 Units

- Mass: kilograms (`_kg` suffixes). Density: kg/m³. Lengths: meters (`_m`).
- Angles: degrees (`_deg` suffixes) in joint limits. Stiffness: pascals (`_pa` suffixes).

### 4.2 Center of mass

`center_of_mass` is normalized to the referenced geometry's **local bounding box**: each coordinate ∈ `[0,1]`, where `0` is the box minimum and `1` the box maximum along that axis. `[0.5, 0.5, 0.5]` is the geometric center. This keeps the value invariant across tooling that changes units or scale.

## 5. Identifiers

- `metadata.document_id`: any URI (`scheme:rest`). Issuers SHOULD use a URN or a resolvable HTTPS URI.
- `objects[].id`: a document-local identifier. It MUST be unique within the document, and — when embedding into glTF (Section 8) — MUST equal the target node's `name`.
- `material_id` and `part_id`: local to the object; `BomEntry.material_id` MUST resolve to a `MaterialFraction.material_id` in the same object.

## 6. Validation rules (normative)

A document is **valid** iff it produces zero errors. Issues are `{ code, path, message }`; `path` is a JSON Pointer into the document. Errors are sorted by `(path, code, message)` before reporting; warnings likewise.

The reference validator enforces the published schema in two layers: a **structural subset** of JSON Schema (types, `required`, `enum`, `const`, `items`, `additionalProperties: false`, `$ref`) reported as `V-01`, and a **semantic layer** that enforces every value-level constraint (bounds, patterns, sizes, cross-field rules) with its own code. The two layers together are equivalent to validating against the full schema; binary formats (Section 7.3–7.4), which have no schema wire-form, are validated by the semantic layer plus `V-01` twins of the structural rules.

### Errors

| Code | Rule |
|---|---|
| `V-01` | Structural conformance: required fields present, JSON types correct, enum values known, `spec_version` is exactly `"0.1.0"`, the objects/BOM/composition arrays are non-empty, and joint-limit objects use only declared keys. In protobuf, unknown enum strings and a missing `metadata` message are reported here as well. |
| `V-02` | Mass fields are finite and `> 0`: `physical.mass_kg`, `bill_of_materials[].mass_kg` (when present). |
| `V-03` | `friction.static` and, if present, `friction.kinetic` are finite and `≥ 0`. |
| `V-04` | `stiffness.youngs_modulus_pa` finite `> 0`; `shear_modulus_pa` and `density_kg_m3` (if present) finite `> 0`; `poisson_ratio` (if present) finite in `[0, 0.5)`. |
| `V-05` | Every `center_of_mass` coordinate is in `[0,1]`; exactly 3 coordinates. |
| `V-06` | For each joint: `axis` (unless `fixed`) is a 3-vector with `|‖axis‖ − 1| ≤ 1e-6`; if `limits` present, the keys match the joint type and `lower ≤ upper`. |
| `V-07` | Uniqueness: object `id`s in the document; joint `id`s per object; `part_id`s per object; `material_id`s per object. |
| `V-08` | Each joint's `parent`/`child` resolve to object `id`s in the document, and `parent ≠ child`. |
| `V-09` | Material fractions: each in `(0,1]`, and `|Σ fractions − 1| ≤ 0.001`. |
| `V-10` | Each `BomEntry.material_id` (if present) resolves to a `material_composition[].material_id` of the same object. |
| `V-11` | `recyclability_rate` and, if present, `repairability_score` are in `[0,1]`. |
| `V-12` | Disassembly steps are strictly increasing starting at 1, and each `target` resolves to a `part_id` or object `id` of the document. |
| `V-13` | Geometry hashes match their algorithm: `sha256` → 64 lowercase hex chars, `sha512` → 128. |
| `V-14` | All timestamp fields parse as RFC 3339 (`created`, `cra.support_period_until`, `cra.logs[].timestamp`). |
| `V-15` | `quantity ≥ 1`; `scale_hint_m` has exactly 3 entries, each `> 0`. |
| `V-16` | URI fields are well-formed (`scheme:rest`, scheme starts with a letter and continues with `A-Za-z0-9+.-`): `document_id`, `geometry.uri`, `sbom_ref`, `hbom_ref`, `plan_ref`, `vulnerability_disclosure_uri`. |
| `V-17` | Required strings are non-empty: `metadata.name`, object `id`/`label`, joint `id`, `part_id`, part `label`, `material_id`, material `label`, `passport_id`, CRA log `event`. |

### Warnings (do not affect validity)

| Code | Rule |
|---|---|
| `W-01` | A `geometry.hash` is absent: asset integrity cannot be verified. |
| `W-02` | `cra.conformance` is `unknown`. |
| `W-03` | `compliance.disassembly` is absent: no machine-readable take-apart guidance for rescue/recycling robots. |

## 7. Serializations

All four serializations represent the same model. Round-trips between any pair of formats MUST be semantically equal (same canonical JSON, Section 7.1).

### 7.1 Canonical JSON

- UTF-8, object keys sorted by code point, no insignificant whitespace.
- Integers stay integers; reals use the shortest decimal that round-trips.
- `$schema`, `@context`, `@type` are transport annotations: parsers MUST accept and ignore them; the canonical form does not emit them.

### 7.2 JSON-LD

Identical to canonical JSON plus, at the document root:

```json
{
  "@context": "https://spatial-dpp.org/contexts/v0.1.0",
  "@type": "SpatialDppDocument",
  "metadata": { ... },
  "objects": [ ... ]
}
```

The context (in `schemas/v0.1.0/context.jsonld`) maps field names to `https://spatial-dpp.org/terms/` IRIs so compliance graphs can be joined with other linked-data datasets.

### 7.3 MessagePack

The canonical model encoded with MessagePack (via `rmp-serde` semantics: struct fields as map keys). Suitable for compact transport to embedded and robotics consumers.

### 7.4 Protocol Buffers

`schemas/v0.1.0/spatial_dpp.proto` (proto3). Design decisions:

- Enumerated values are carried as **strings** (not proto enums) for forward compatibility: readers MUST NOT fail on unknown enum values during binary decode; the semantic layer reports them as `V-01`.
- Where JSON marks a field required, proto marks the corresponding message field `optional` (message presence) or checks emptiness after decode; the semantic layer enforces presence so binary documents cannot bypass the JSON Schema rules.
- `quantity`/`order` are `uint32`; `V-15` still enforces `≥ 1` because proto3 default `0` is indistinguishable from absent.

### 7.5 Format detection

Byte-stream consumers may omit an explicit format. Detection order: JSON (first non-whitespace byte `{`), MessagePack (a probe decode must yield a top-level map), then Protobuf. A protobuf document always starts with field tag `0x0a` (field 1, wire type 2), which no map-shaped MessagePack document begins with, so the probe order is unambiguous.

## 8. glTF 2.0 extension `EXT_spatial_dpp`

Full text: `docs/gltf-extension/EXT_spatial_dpp.md`. Summary:

- `"EXT_spatial_dpp"` is added to `extensionsUsed`.
- `asset.extensions.EXT_spatial_dpp.metadata` carries `DocumentMetadata`.
- Each glTF node whose `name` equals an object's `id` receives `node.extensions.EXT_spatial_dpp.object` with the full `SpatialObject`.
- Extraction reverses the process; a missing `geometry.locator` is auto-filled with `#/nodes/<index>`.
- v0.1.0 supports JSON-encoded `.gltf` only (`.glb` is on the roadmap).

## 9. USD export mapping

`export --target=usd` emits a `.usda` layer. Mapping table:

| Spatial DPP | USD |
|---|---|
| object (per `id`) | `def Xform "<id>"` under `/SpatialDPP`, with `PhysicsRigidBodyAPI` |
| `physical.mass_kg` | `double physics:mass` |
| `physical.center_of_mass` | `double3 sdpp:centerOfMassNormalized` (bbox-relative; see 4.2) |
| `physical.friction.static/kinetic` | `double sdpp:frictionStatic` / `sdpp:frictionKinetic` |
| `physical.density_kg_m3` | `double sdpp:densityKgM3` |
| `physical.stiffness.*` | `double sdpp:youngsModulusPa` / `sdpp:shearModulusPa` / `sdpp:poissonRatio` |
| `joints[type=revolute]` | `def PhysicsRevoluteJoint` with `physics:lower/upperLimit` (degrees) and `rel physics:body0/1`; arbitrary axes as `double3 sdpp:axis`, canonical +X/+Y/+Z axes additionally as `uniform token physics:axis` |
| `joints[type=prismatic]` | `def PhysicsPrismaticJoint`, same relation/limit mapping with meter limits |
| `joints[type=spherical|fixed]` | `def Xform "joint_<id>"` with `custom string sdpp:jointType` and `double3 sdpp:axis` |
| `geometry`, `compliance` | `custom dictionary sdpp:*` (deterministic key order) |
| `metadata` | layer `customLayerData` |

Output is byte-deterministic for equal documents (golden-tested).

## 10. Zero-copy parsing semantics

The Rust core parses by borrowing wherever the wire format permits: string fields (`uri`, `hash.value`, ids, labels) are deserialized as borrowed `&str`/`Cow<[u8]>` from the input buffer when lifetimes allow, and binary formats are decoded without staging an intermediate DOM. "Zero-copy" here means: no allocation proportional to the source for the hot fields consumed by physics loops; a full owned `SpatialDppDocument` is produced only when the caller asks for it.

## 11. Extensibility and versioning

- The schema allows unknown keys everywhere (`additionalProperties: true`), so `x-*` vendor extensions never break validation. The v0.1.0 reference implementation ignores them when parsing; preservation is planned for v0.2.
- Schema versioning follows SemVer under `schemas/vX.Y.Z/`; additive changes bump minor, breaking changes bump major. `spec_version` is pinned per document and checked by `V-01`.
- Unknown enum values in JSON fail `V-01`; unknown enum values in protobuf decode succeed structurally and fail `V-01` semantically (Section 7.4).

## 12. OD-BOM interop

`compliance.disassembly.plan_ref` is intended to carry a URI to an OD-BOM document (DeCAD). When present, the embedded `steps` array is the summary form; OD-BOM remains the authoritative constraint graph (fasteners, interferences, precedence) for planners.
