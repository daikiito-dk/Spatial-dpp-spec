# EXT_spatial_dpp

**Extension name:** `EXT_spatial_dpp`
**Status:** draft (spatial-dpp v0.1.0)
**glTF version:** 2.0 (JSON-encoded `.gltf` only in v0.1.0; `.glb` is planned)
**Dependencies:** none

## 1. Motivation

glTF is the dominant interchange format for real-time 3D, but it has no standard place for the physical and regulatory truth about an object: mass, center of mass, friction, stiffness, joints, material composition, recyclability, disassembly steps, and EU Digital Product Passport / CRA records. This extension embeds a full `SpatialDppDocument` (see `../../SPEC.md`) into a glTF scene so that simulators, robots, and auditors read one file and get geometry **and** its physics **and** its compliance data — without breaking existing loaders, which ignore unknown extensions.

## 2. Design

- **One glTF node = one spatial object.** The extension object is attached per node. The node's `name` MUST equal the object's `id`.
- **Document metadata lives once**, on the asset.
- **The object's `geometry.uri` becomes optional knowledge**: the node itself is (part of) the geometry. Extractors auto-fill `geometry.locator` with `#/nodes/<index>` when absent.
- Loaders that do not support the extension must still load the file cleanly (glTF spec: unknown extensions in `extensionsUsed` are ignored at the cost of undefined behavior only for assets that *require* them; this extension never appears in `extensionsRequired`).

## 3. Schema

### glTF root

```json
{
  "asset": {
    "version": "2.0",
    "extensions": {
      "EXT_spatial_dpp": {
        "metadata": { "spec_version": "0.1.0", "document_id": "...", "name": "...", "created": "..." }
      }
    }
  },
  "extensionsUsed": ["EXT_spatial_dpp"],
  "nodes": [
    {
      "name": "ev_pack",
      "extensions": {
        "EXT_spatial_dpp": {
          "object": {
            "id": "ev_pack",
            "label": "EV Battery Pack",
            "geometry": { "format": "gltf", "uri": "urn:example:pack-mesh", "hash": { "algorithm": "sha256", "value": "..." } },
            "physical": { "...": "see spatial-dpp schema" },
            "compliance": { "...": "see spatial-dpp schema" }
          }
        }
      }
    }
  ]
}
```

### Properties

| Location | Property | Type | Required |
|---|---|---|---|
| `asset.extensions.EXT_spatial_dpp` | `metadata` | `DocumentMetadata` | yes |
| `node.extensions.EXT_spatial_dpp` | `object` | `SpatialObject` | yes |
| `node.extensions.EXT_spatial_dpp.object` | full `SpatialObject` per spatial-dpp v0.1.0 schema | — | yes |

## 4. Embedding rules (writers)

1. Add `EXT_spatial_dpp` to `extensionsUsed` (never `extensionsRequired`).
2. Write `asset.extensions.EXT_spatial_dpp.metadata` exactly once.
3. For every object in the document, find the glTF node whose `name` equals `object.id`. If none exists, embedding MUST fail (error, not silent skip).
4. Write the complete object, including its `geometry`, `physical`, and `compliance` layers, to that node's `EXT_spatial_dpp.object`. Keep the object's `geometry.uri`; it identifies the authoritative asset even when the node carries mesh primitives directly.
5. Output key order is the canonical JSON order (sorted keys) for determinism.

## 5. Extraction rules (readers)

1. Read `asset.extensions.EXT_spatial_dpp.metadata`; absence is an error.
2. For each node (in array order) with `EXT_spatial_dpp.object`, parse a `SpatialObject`.
3. If `geometry.locator` is absent, set it to `#/nodes/<node index>`.
4. Validate the reconstructed document with the spatial-dpp validator (structural + semantic layers).
5. Duplicate object `id`s across nodes produce a `V-07` error at validation time.

## 6. Example

See `tests/golden/valid/ev_battery_pack.json` (document) and the glTF round-trip test in `crates/sdpp-core/tests/`, which embeds and re-extracts a document from a minimal glTF scene.

## 7. Known limitations (v0.1.0)

- JSON `.gltf` only; `.glb` (binary container) is not supported yet.
- The extension intentionally duplicates `geometry.uri` even though the node holds geometry; the URI plus hash remain the integrity anchor for supply-chain use.
- glTF scene-graph transforms are not reconciled with bbox-normalized `center_of_mass`; consumers apply node transforms as needed (SPEC.md 4.2 defines the normalization convention).
