# spatial-dpp-spec

[![CI](https://github.com/daikiito-dk/Spatial-dpp-spec/actions/workflows/ci.yml/badge.svg)](https://github.com/daikiito-dk/Spatial-dpp-spec/actions/workflows/ci.yml)
[![License: Apache 2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Spec](https://img.shields.io/badge/spec-v0.1.0-draft-orange.svg)](SPEC.md)

**English** | [日本語](README.ja.md) | [Documentation site](https://daikiito-dk.github.io/Spatial-dpp-spec/)

**spatial-dpp** is an open, machine-readable data structure that binds, for one physical object:

1. **Geometry** — a non-destructive reference (URI + content hash) into glTF / USD / STEP / OBJ / STL assets
2. **Physical properties** — mass, center of mass, friction, stiffness, and kinematic joints, in the exact units a physics engine consumes
3. **Compliance & DPP** — bill of materials, material composition, recyclability, disassembly steps, EU Digital Product Passport (SBOM/HBOM), and Cyber Resilience Act records

…so that a simulator, a robot, or an auditor can answer from a single parse: *what is this object, how does it behave physically, and how do I take it apart safely and lawfully?*

Shipped as a Rust reference implementation (`sdpp-core`), a CLI (`spatial-dpp`), and WASM/TypeScript bindings (`sdpp-wasm`).

## The format in 30 seconds

```json
{
  "metadata": {
    "spec_version": "0.1.0",
    "document_id": "urn:product:ev-battery-pack-2026",
    "name": "Generic EV Battery Pack Module A",
    "created": "2026-09-01T00:00:00Z"
  },
  "objects": [
    {
      "id": "ev_pack",
      "label": "EV Battery Pack",
      "geometry": {
        "format": "gltf",
        "uri": "urn:example:meshes/ev-pack-v3",
        "hash": { "algorithm": "sha256", "value": "3fa85f64…" },
        "locator": "#/nodes/0"
      },
      "physical": {
        "mass_kg": 12.4,
        "center_of_mass": [0.5, 0.5, 0.41],
        "friction": { "static": 0.6, "kinetic": 0.4 },
        "stiffness": { "youngs_modulus_pa": 69000000000.0 },
        "joints": [
          { "id": "hinge_lid", "type": "revolute", "axis": [0.0, 0.0, 1.0],
            "limits": { "lower_deg": -45.0, "upper_deg": 45.0 },
            "parent": "ev_pack", "child": "pack_lid" }
        ]
      },
      "compliance": {
        "bill_of_materials": [
          { "part_id": "m6_screw", "label": "M6 Torx Head Screw", "quantity": 8,
            "material_id": "steel_galv" }
        ],
        "material_composition": [
          { "material_id": "li_ion_cells", "label": "Lithium-Ion Cells", "fraction": 0.55 }
        ],
        "recyclability_rate": 0.62,
        "dpp": {
          "passport_id": "urn:dpp:eu:ev-pack-2026",
          "sbom_ref": "https://example.com/sbom/ev-pack-2026.json",
          "hbom_ref": "urn:hbom:ev-pack-2026"
        },
        "disassembly": {
          "steps": [
            { "order": 1, "action": "unfasten", "target": "m6_screw", "tool": "torx_t30" },
            { "order": 3, "action": "deactivate", "target": "cell_array",
              "hazard": "thermal_runaway" }
          ]
        },
        "cra": { "conformance": "conformant",
                 "support_period_until": "2032-09-01T00:00:00Z" }
      }
    }
  ]
}
```

Units live in field names (`mass_kg`, `lower_deg`, `youngs_modulus_pa`); center of mass is bbox-normalized `[0,1]` so the value survives unit and scale changes (SPEC.md 4.2).

## Validation

Two layers, one report. Structural conformance is `V-01`; every value rule has its own code — `V-02` (mass `> 0`), `V-05` (COM in `[0,1]`), `V-06` (unit-length joint axes), `V-09` (material fractions sum to `1 ± 0.001`), `V-12` (executable disassembly steps), `V-13` (hash integrity), `V-14` (RFC 3339 timestamps), and more. Binary formats get identical reports because every structural rule has a semantic twin. Full table: SPEC.md Section 6.

```json
{
  "valid": false,
  "errors": [
    { "code": "V-09", "path": "/objects/0/compliance/material_composition",
      "message": "material fractions must sum to 1 within ±0.001 (got 0.94)" }
  ],
  "warnings": [
    { "code": "W-03", "path": "/objects/1/compliance/disassembly",
      "message": "compliance has no disassembly plan; rescue/recycling robots will lack machine-readable take-apart guidance" }
  ],
  "document_id": "urn:product:ev-battery-pack-2026",
  "object_count": 2
}
```

## Build and install

Requires Rust 1.75+ (no protoc needed):

```bash
cargo build --release          # spatial-dpp binary in target/release/
cargo test --workspace         # 59 tests: golden, boundary, round-trip, CLI
```

WASM/TypeScript:

```bash
rustup target add wasm32-unknown-unknown
wasm-pack build crates/sdpp-wasm --out-dir pkg --target web
```

## CLI

```bash
$ spatial-dpp validate examples/ev_battery_pack.json
{"valid":true,"errors":[],"warnings":[...],"document_id":"urn:product:ev-battery-pack-2026","object_count":2}

$ spatial-dpp validate tests/golden/invalid/negative_mass.json   # exit 1
{"valid":false,"errors":[{"code":"V-02","path":"/objects/0/physical/mass_kg",...}],"object_count":1}

$ spatial-dpp inspect examples/ev_battery_pack.json
Document:   Generic EV Battery Pack Module A
ID:         urn:product:ev-battery-pack-2026
...
[ev_pack] EV Battery Pack
  Mass:        12.4 kg
  Recyclable:  62.0%
  Disassembly: 3 steps: 1 unfasten(m6_screw); 2 lift(cover_top); 3 deactivate(cell_array)

$ spatial-dpp export examples/ev_battery_pack.json --target=usd --out pack.usda
$ spatial-dpp convert examples/ev_battery_pack.json --to msgpack --out pack.sdppmsg
$ spatial-dpp gltf-embed examples/ev_battery_pack.json scene.gltf --out scene.with-dpp.gltf
$ spatial-dpp gltf-extract scene.with-dpp.gltf
```

Exit codes: `0` valid / success, `1` document invalid, `2` usage or parse error.

## Formats

| Serialization | Notes |
|---|---|
| Canonical JSON | sorted keys, shortest round-trip decimals; transport keys (`$schema`, `@context`) are ignored |
| JSON-LD | canonical JSON + `@context`/`@type` (`schemas/v0.1.0/context.jsonld`), joins compliance graphs with linked data |
| MessagePack | structs as maps; compact transport for embedded/robotics consumers |
| Protobuf | `schemas/v0.1.0/spatial_dpp.proto`; enums as strings for forward compatibility |
| glTF 2.0 | `EXT_spatial_dpp` extension, one node = one object (`docs/gltf-extension/EXT_spatial_dpp.md`) |
| USDA | deterministic export mapping to UsdPhysics attributes (`export --target=usd`) |

Round-trips between any pair are semantic-identical and covered by tests.

## Repository layout

```
Spatial-dpp-spec/
├── AGENTS.md                # guidance for coding agents (Factory Droid et al.)
├── SPEC.md                  # normative data model, rules, formats (SSOT)
├── schemas/v0.1.0/          # JSON Schema, Protobuf, JSON-LD context
├── crates/
│   ├── sdpp-core/           # types, 2-layer validation, codecs, glTF, USD, inspect
│   ├── sdpp-cli/            # `spatial-dpp` binary
│   └── sdpp-wasm/           # WASM/TypeScript bindings (+ typescript/index.d.ts)
├── docs/                    # GitHub Pages site + glTF extension spec
├── examples/                # sample documents (sync-tested against golden fixtures)
└── tests/golden/            # deterministic fixtures with golden reports / USDA / inspect
```

## Why this matters

- **Physical AI and simulators** read one file and get collision-relevant physics: mass, COM, friction, stiffness, and joints — bbox-normalized so tooling can remap units.
- **Disaster-response and recycling robots** get machine-readable take-apart plans with hazards (`thermal_runaway`, `asbestos`, …) per step, and can link to full OD-BOM constraint graphs from the sister project [DeCAD](https://github.com/daikiito-dk/DeCAD) via `disassembly.plan_ref`.
- **Compliance teams** carry EU Digital Product Passport and CRA records alongside the geometry they describe, so an asset's regulatory truth travels with the asset.

## Roadmap

- **Phase 1 (this release, v0.1.0):** schema + spec, reference validator (structural + semantic), JSON/JSON-LD/MessagePack/Protobuf codecs, glTF `EXT_spatial_dpp`, USDA export, CLI, WASM bindings, deterministic test suite, CI.
- **Phase 2:** `.glb` container support, preservation of `x-*` extensions through round-trips, cryptographic passport signing, USD round-trip (import), glTF validator integration.
- **Phase 3:** propose the stabilized schema to standards and robotics consortiums; ROS 2 integration package.

## Contributing

See [AGENTS.md](AGENTS.md) (it also drives Factory Droid and other coding agents: rules for schema SSOT, determinism, validation codes, and the definition of done). Licensed under [Apache-2.0](LICENSE).
