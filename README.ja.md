# spatial-dpp-spec

[![CI](https://github.com/daikiito-dk/Spatial-dpp-spec/actions/workflows/ci.yml/badge.svg)](https://github.com/daikiito-dk/Spatial-dpp-spec/actions/workflows/ci.yml)
[![License: Apache 2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Spec](https://img.shields.io/badge/spec-v0.1.0-draft-orange.svg)](SPEC.md)

[English](README.md) | **日本語** | [ドキュメントサイト](https://daikiito-dk.github.io/Spatial-dpp-spec/)

**spatial-dpp** は、1つの物理オブジェクトについて以下の3層を1つの機械可読ドキュメントに束ねるオープンなデータ構造です。

1. **ジオメトリ** — glTF / USD / STEP / OBJ / STL への非破壊的参照（URI + コンテンツハッシュ）
2. **物理属性** — 質量・重心・摩擦・剛性・関節（可動域）。物理シミュレータがそのまま消費できる単位・形式
3. **コンプライアンス & DPP** — BOM、素材構成、リサイクル率、分解手順、EUデジタルプロダクトパスポート（SBOM/HBOM）、CRA（サイバー耐性法）記録

これにより、シミュレータ・ロボット・監査人は1回のパースで「この物体は何で、物理的にどう振る舞い、どう安全・適法に分解できるか」を答えられます。

Rustリファレンス実装（`sdpp-core`）、CLI（`spatial-dpp`）、WASM/TypeScriptバインディング（`sdpp-wasm`）を同梱します。

## 30秒で分かるフォーマット

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

単位はフィールド名に埋め込み（`mass_kg`、`lower_deg`、`youngs_modulus_pa`）。重心はバウンディングボックス正規化 `[0,1]` なので、単位やスケールが変わっても値が生き残ります（SPEC.md 4.2）。

## 検証（バリデーション）

2層構造で1つのレポートを出します。構造は `V-01`、値のルールにはそれぞれコードがあります — `V-02`（質量 `> 0`）、`V-05`（重心 `[0,1]`）、`V-06`（関節軸は単位ベクトル）、`V-09`（素材比率の合計 `1 ± 0.001`）、`V-12`（分解ステップの実行可能性）、`V-13`（ハッシュ整合性）、`V-14`（RFC 3339 タイムスタンプ）ほか。バイナリ形式もJSONと同一のレポートになります（構造ルールには全てセマンティック側の双子が存在するため）。全表: SPEC.md セクション6。

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

## ビルド

Rust 1.75+（protoc 不要）:

```bash
cargo build --release          # target/release/spatial-dpp
cargo test --workspace         # 59テスト: golden / boundary / round-trip / CLI
```

WASM/TypeScript:

```bash
rustup target add wasm32-unknown-unknown
wasm-pack build crates/sdpp-wasm --out-dir pkg --target web
```

## CLI

```bash
$ spatial-dpp validate examples/ev_battery_pack.json     # exit 0
$ spatial-dpp validate tests/golden/invalid/negative_mass.json   # exit 1, V-02
$ spatial-dpp inspect examples/ev_battery_pack.json     # 人間可読サマリ
$ spatial-dpp export examples/ev_battery_pack.json --target=usd --out pack.usda
$ spatial-dpp convert examples/ev_battery_pack.json --to msgpack --out pack.sdppmsg
$ spatial-dpp gltf-embed examples/ev_battery_pack.json scene.gltf --out scene.with-dpp.gltf
$ spatial-dpp gltf-extract scene.with-dpp.gltf
```

終了コード: `0` 有効/成功、`1` ドキュメント無効、`2` 使用法・パースエラー。

## フォーマット

| シリアライズ | 備考 |
|---|---|
| 正準JSON | ソート済みキー、最短ラウンドトリップ小数 |
| JSON-LD | `@context`/`@type` 付き。リンクトデータとしてコンプライアンスグラフを連結可能 |
| MessagePack | struct をマップとして直列化。組込み・ロボティクス向けの軽量転送 |
| Protobuf | `schemas/v0.1.0/spatial_dpp.proto`。列挙は前方互換のため文字列 |
| glTF 2.0 | `EXT_spatial_dpp` 拡張。ノード＝オブジェクト |
| USDA | 決定論的エクスポート（UsdPhysics属性へマッピング） |

任意のペアのラウンドトリップが意味等価であることをテストで保証しています。

## リポジトリ構成

```
Spatial-dpp-spec/
├── AGENTS.md                # コーディングエージェント向け規約（Factory Droid等）
├── SPEC.md                  # 規範的データモデル・ルール・形式（SSOT）
├── schemas/v0.1.0/           # JSON Schema / Protobuf / JSON-LD コンテキスト
├── crates/
│   ├── sdpp-core/            # 型、2層検証、コーデック、glTF、USD、inspect
│   ├── sdpp-cli/             # `spatial-dpp` バイナリ
│   └── sdpp-wasm/           # WASM/TypeScript バインディング
├── docs/                     # GitHub Pages サイト + glTF拡張仕様
├── examples/                 # サンプルドキュメント（golden fixtureと同一であることをテスト）
└── tests/golden/             # 決定論的フィクスチャ + golden レポート/USDA/inspect
```

## なぜ重要か

- **フィジカルAI・シミュレータ**: 1つのファイルから質量・重心・摩擦・剛性・関節を直接読める。
- **災害対応・リサイクルロボット**: ステップ単位の危険フラグ（`thermal_runaway`、`asbestos` など）付きの分解手順を機械可読で取得し、`disassembly.plan_ref` 経由で姉妹プロジェクト [DeCAD](https://github.com/daikiito-dk/DeCAD) の OD-BOM 制約グラフへ接続できる。
- **コンプライアンスチーム**: EU DPP と CRA の記録がジオメトリと一体で運搬され、資産の規制上の真実が資産と共に移動する。

## ロードマップ

- **フェーズ1（本リリース v0.1.0）:** スキーマと仕様、リファレンスバリデータ、JSON/JSON-LD/MessagePack/Protobuf、glTF `EXT_spatial_dpp`、USDAエクスポート、CLI、WASM、決定論的テスト、CI。
- **フェーズ2:** `.glb` 対応、`x-*` 拡張キーの保存、パスポートの電子署名、USDインポート（ラウンドトリップ）。
- **フェーズ3:** 安定化したスキーマの標準化団体・ロボティクスコンソーシアムへの提案、ROS 2 統合パッケージ。

## コントリビューション

[AGENTS.md](AGENTS.md) を参照（Factory Droid 等のコーディングエージェント向け規約: スキーマSSOT、決定論性、検証コード運用、完了条件）。ライセンス: [Apache-2.0](LICENSE)。
