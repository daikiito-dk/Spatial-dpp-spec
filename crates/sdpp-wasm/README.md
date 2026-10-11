# sdpp-wasm — TypeScript / WASM bindings

WASM bindings for `sdpp-core`, for browser and Node.js consumers that need to
validate, inspect, and convert Spatial DPP documents without a Rust toolchain
at runtime.

## Build

```bash
rustup target add wasm32-unknown-unknown
wasm-pack build crates/sdpp-wasm --out-dir pkg --target web
```

`pkg/` is generated output (gitignored). The hand-written TypeScript type
surface lives in `typescript/index.d.ts`.

## Usage (web)

```ts
import init, { validate_json, inspect_json } from "spatial-dpp";

await init();
const report = JSON.parse(validate_json(docJson));
if (!report.valid) {
  for (const issue of report.errors) {
    console.error(`${issue.code} at ${issue.path}: ${issue.message}`);
  }
} else {
  console.log(inspect_json(docJson));
}
```

## API

| Export | Signature | Notes |
|---|---|---|
| `version` | `() => string` | e.g. `spatial-dpp 0.1.0` |
| `validate_json` | `(src: string) => string` | Always returns a report JSON; parse failures surface as `V-01`. |
| `inspect_json` | `(src: string) => string` | Human-readable summary. |
| `to_jsonld` | `(src: string) => string` | Canonical JSON + `@context`/`@type`. |
| `to_msgpack` / `from_msgpack` | `string ⇄ Uint8Array` | Compact binary transport. |
| `to_protobuf` / `from_protobuf` | `string ⇄ Uint8Array` | Binary transport per `schemas/v0.1.0/spatial_dpp.proto`. |

CI verifies the crate compiles for `wasm32-unknown-unknown` on every push.
