# AGENTS.md — Guidance for Factory Droid and other coding agents

This file tells autonomous agents how to work in this repository. `SPEC.md` is the single source of truth; when code and `SPEC.md` disagree, fix the code (or, if the spec itself is wrong, change `SPEC.md` in the same commit and record why).

## Repository layout

```
Spatial-dpp-spec/
├── AGENTS.md                 # this file
├── SPEC.md                   # normative data model + rules (SSOT)
├── schemas/v0.1.0/           # JSON Schema, Protobuf, JSON-LD context (canonical, embedded by the core at compile time)
├── docs/                     # index.html (GitHub Pages site) + gltf-extension spec
├── crates/
│   ├── sdpp-core/            # types, validation, codecs (JSON/JSON-LD/MessagePack/Protobuf), glTF embed/extract, USD export
│   ├── sdpp-cli/             # `spatial-dpp` binary: validate / inspect / export / convert / gltf-embed / gltf-extract
│   └── sdpp-wasm/            # wasm-bindgen bindings for TypeScript (typescript/ holds the hand-written index.d.ts; pkg/ is generated)
├── examples/                 # sample documents (kept identical to tests/golden/valid by a sync test)
└── tests/golden/             # deterministic fixtures: valid/ + invalid/ (+ *.expected code lists)
```

## Build and test commands

```bash
cargo test --workspace        # all unit + golden + boundary + round-trip tests
cargo fmt --all               # run before committing
cargo clippy --all-targets -- -D warnings
cargo run -p sdpp-cli -- validate examples/ev_battery_pack.json
cargo check -p sdpp-wasm --target wasm32-unknown-unknown   # after `rustup target add wasm32-unknown-unknown`
```

CI (`.github/workflows/ci.yml`) runs fmt, clippy, tests, and the wasm32 check. All four must pass; do not weaken a test to make it green.

## Hard rules

1. **Do not duplicate the schema.** The JSON Schema is embedded at compile time via `include_str!` from `schemas/v0.1.0/`. Never copy it into `crates/`. The protobuf structs in `crates/sdpp-core/src/proto.rs` are the only hand-maintained mirror of `schemas/v0.1.0/spatial_dpp.proto`; if you change either, change both in the same commit. The round-trip tests will catch drift.
2. **Determinism is a contract.** Canonical JSON uses sorted keys; validation reports, inspect output, and USD export are golden-tested byte-for-byte. Never introduce HashMap iteration, system time, or randomness into any serializer or test expectation.
3. **Validation codes are public API.** `V-01`…`V-15` and `W-01`…`W-03` are defined in `SPEC.md` Section 6. Adding a rule means: add it to `SPEC.md`, implement it in `validate.rs`, add a fixture in `tests/golden/invalid/` with its `.expected` file, and update the table. Renumbering existing codes is a breaking change (major version).
4. **Error messages must state path and value.** Validation issues always carry a JSON Pointer path and a message that includes the offending value. Errors must never panic; use `Result`/`Option` in library code.
5. **Two validation layers.** The reference validator implements only the structural subset of the published JSON Schema (`type`, `required`, `enum`, `const`, `items`, `additionalProperties: false`, `$ref`), reported as `V-01`. Every value-level constraint (bounds, patterns, sizes, cross-field rules) lives in the semantic layer with its own `V-xx` code. Binary formats (MessagePack, Protobuf) have no schema wire-form, so every structural rule also has a semantic `V-01` twin (empty arrays, unknown enum strings, missing protobuf `metadata`). If you add a schema constraint, add the semantic twin with a SPEC.md code in the same change.
6. **Units are in field names.** `mass_kg`, `lower_deg`, `youngs_modulus_pa` — never introduce unitless numeric names. Center of mass is bbox-normalized `[0,1]` (SPEC.md 4.2), not meters.
7. **Tolerances are normative.** Fraction sum `±0.001`, axis unit `±1e-6`. Golden boundary tests pin them; do not loosen a tolerance without bumping the spec version.
8. **Extensibility.** Unknown `x-*` keys must never cause a validation failure (schema allows them). v0.1.0 ignores them on parse; do not silently start preserving them mid-minor-version.
9. **Commit style.** Imperative subject line ≤ 72 chars, body explains why. One logical change per commit. Never commit generated artifacts (target/, pkg/*.wasm).
10. **CLI contract.** Exit codes: `0` valid/success, `1` document invalid, `2` usage or parse error. The CLI has no non-core dependencies; keep arg parsing hand-rolled in `sdpp-cli/src/lib.rs` and covered by `tests/cli.rs`.

## Definition of done for any change

- [ ] `cargo test --workspace` green, including new fixtures
- [ ] `cargo fmt --all` and `cargo clippy --all-targets -- -D warnings` clean
- [ ] Golden outputs regenerated and reviewed (delete + regenerate, never hand-edit)
- [ ] `SPEC.md` updated if any rule, field, tolerance, or code changed
- [ ] If schema files changed: bump `schemas/` version per Section 11 of `SPEC.md`
