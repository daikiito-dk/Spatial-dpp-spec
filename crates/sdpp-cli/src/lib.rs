//! Hand-rolled CLI for the spatial-dpp toolkit (AGENTS.md rule 10).
//!
//! Exit codes: `0` valid/success, `1` document invalid, `2` usage or parse error.

use std::io::Write as _;

use sdpp_core::format::{self, Format};
use sdpp_core::validate::{validate_bytes, ValidationReport};
use sdpp_core::{gltf, inspect, usd};

pub const NAME: &str = "spatial-dpp";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub struct CliOutput {
    pub code: i32,
    /// Raw stdout bytes (binary formats write here unchanged).
    pub stdout: Vec<u8>,
    pub stderr: String,
}

/// Run the CLI against arguments and print to the real streams.
pub fn run(args: &[String]) -> i32 {
    let output = run_with_output(args);
    if !output.stdout.is_empty() {
        let mut stdout = std::io::stdout();
        let _ = stdout.write_all(&output.stdout);
        let _ = stdout.flush();
    }
    if !output.stderr.is_empty() {
        eprint!("{}", output.stderr);
    }
    output.code
}

/// Run the CLI and capture output (used by tests and the binary).
pub fn run_with_output(args: &[String]) -> CliOutput {
    let (code, stdout, stderr) = execute(args);
    CliOutput {
        code,
        stdout,
        stderr,
    }
}

fn usage() -> String {
    format!(
        "{NAME} v{VERSION} — Spatial DPP v0.1.0 validator and toolkit

USAGE:
    {NAME} <COMMAND> [ARGS]

COMMANDS:
    validate <file> [--format auto|json|jsonld|msgpack|proto] [--pretty]
            Validate structure, physical values, and compliance rules.
    inspect <file> [--format auto|json|jsonld|msgpack|proto]
            Print a human-readable summary of a document.
    export <file> --target=usd [--out <file>]
            Export to another format (v0.1.0: usd only).
    convert <file> --to msgpack|json|jsonld|proto [--out <file>]
            Convert between serializations.
    gltf-embed <doc.json> <scene.gltf> [--out <file>]
            Embed a document into a glTF scene (EXT_spatial_dpp).
    gltf-extract <scene.gltf> [--out <file>]
            Extract a document from a glTF scene.

EXIT CODES:
    0  valid / success
    1  document invalid
    2  usage or parse error
"
    )
}

fn execute(args: &[String]) -> (i32, Vec<u8>, String) {
    let Some(command) = args.first() else {
        return (2, Vec::new(), usage());
    };
    match command.as_str() {
        "--help" | "-h" | "help" => (0, usage().into_bytes(), String::new()),
        "--version" | "-v" | "version" => {
            (0, format!("{NAME} {VERSION}\n").into_bytes(), String::new())
        }
        "validate" => cmd_validate(&args[1..]),
        "inspect" => cmd_inspect(&args[1..]),
        "export" => cmd_export(&args[1..]),
        "convert" => cmd_convert(&args[1..]),
        "gltf-embed" => cmd_gltf_embed(&args[1..]),
        "gltf-extract" => cmd_gltf_extract(&args[1..]),
        other => (
            2,
            Vec::new(),
            format!("unknown command `{other}`\n\n{}", usage()),
        ),
    }
}

// --- tiny flag parser --------------------------------------------------------

fn collect(rest: &[String]) -> (Vec<String>, Vec<(String, Option<String>)>) {
    let mut positional = Vec::new();
    let mut flags = Vec::new();
    let mut i = 0;
    while i < rest.len() {
        let arg = rest[i].as_str();
        if let Some(name) = arg.strip_prefix("--") {
            if let Some((key, value)) = name.split_once('=') {
                flags.push((key.to_string(), Some(value.to_string())));
            } else if matches!(name, "format" | "out" | "to" | "target")
                && i + 1 < rest.len()
                && !rest[i + 1].starts_with("--")
            {
                flags.push((name.to_string(), Some(rest[i + 1].clone())));
                i += 1;
            } else {
                flags.push((name.to_string(), None));
            }
        } else {
            positional.push(arg.to_string());
        }
        i += 1;
    }
    (positional, flags)
}

fn flag_value<'a>(flags: &'a [(String, Option<String>)], name: &str) -> Option<&'a str> {
    flags
        .iter()
        .find(|(key, _)| key == name)
        .and_then(|(_, value)| value.as_deref())
}

fn flag_present(flags: &[(String, Option<String>)], name: &str) -> bool {
    flags.iter().any(|(key, _)| key == name)
}

fn parse_format_arg(arg: Option<&str>) -> Result<Option<Format>, String> {
    match arg {
        None | Some("auto") => Ok(None),
        Some(s) => Format::parse(s)
            .map(Some)
            .ok_or_else(|| format!("unknown --format `{s}`")),
    }
}

fn read_file(path: &str) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|e| format!("cannot read `{path}`: {e}"))
}

fn write_output(data: &[u8], out: Option<&str>) -> Result<Vec<u8>, String> {
    match out {
        Some(path) => {
            std::fs::write(path, data).map_err(|e| format!("cannot write `{path}`: {e}"))?;
            Ok(Vec::new())
        }
        None => Ok(data.to_vec()),
    }
}

fn report_to_json(report: &ValidationReport, pretty: bool) -> String {
    if pretty {
        serde_json::to_string_pretty(report).unwrap_or_else(|_| "{}".into())
    } else {
        serde_json::to_string(report).unwrap_or_else(|_| "{}".into())
    }
}

// --- commands ----------------------------------------------------------------

fn cmd_validate(rest: &[String]) -> (i32, Vec<u8>, String) {
    let (positional, flags) = collect(rest);
    let Some(file) = positional.first() else {
        return (
            2,
            Vec::new(),
            format!("validate requires <file>\n\n{}", usage()),
        );
    };
    let format_arg = match parse_format_arg(flag_value(&flags, "format")) {
        Ok(f) => f,
        Err(e) => return (2, Vec::new(), format!("{e}\n")),
    };
    let data = match read_file(file) {
        Ok(d) => d,
        Err(e) => return (2, Vec::new(), format!("{e}\n")),
    };
    match validate_bytes(&data, format_arg) {
        Ok(report) => {
            let rendered = report_to_json(&report, flag_present(&flags, "pretty"));
            (
                if report.valid { 0 } else { 1 },
                rendered.into_bytes(),
                String::new(),
            )
        }
        Err(e) => (2, Vec::new(), format!("{e}\n")),
    }
}

fn cmd_inspect(rest: &[String]) -> (i32, Vec<u8>, String) {
    let (positional, flags) = collect(rest);
    let Some(file) = positional.first() else {
        return (
            2,
            Vec::new(),
            format!("inspect requires <file>\n\n{}", usage()),
        );
    };
    let format_arg = match parse_format_arg(flag_value(&flags, "format")) {
        Ok(f) => f,
        Err(e) => return (2, Vec::new(), format!("{e}\n")),
    };
    let data = match read_file(file) {
        Ok(d) => d,
        Err(e) => return (2, Vec::new(), format!("{e}\n")),
    };
    match format::parse_document(&data, format_arg) {
        Ok((_, doc)) => (0, inspect::summary(&doc).into_bytes(), String::new()),
        Err(e) => (2, Vec::new(), format!("{e}\n")),
    }
}

fn cmd_export(rest: &[String]) -> (i32, Vec<u8>, String) {
    let (positional, flags) = collect(rest);
    let Some(file) = positional.first() else {
        return (
            2,
            Vec::new(),
            format!("export requires <file>\n\n{}", usage()),
        );
    };
    let Some(target) = flag_value(&flags, "target") else {
        return (
            2,
            Vec::new(),
            format!("export requires --target=usd\n\n{}", usage()),
        );
    };
    if target != "usd" {
        return (
            2,
            Vec::new(),
            format!("unsupported --target `{target}` (v0.1.0 supports: usd)\n"),
        );
    }
    let data = match read_file(file) {
        Ok(d) => d,
        Err(e) => return (2, Vec::new(), format!("{e}\n")),
    };
    let doc = match format::parse_document(&data, None) {
        Ok((_, doc)) => doc,
        Err(e) => return (2, Vec::new(), format!("{e}\n")),
    };
    let usda = usd::export_usda(&doc);
    match write_output(usda.as_bytes(), flag_value(&flags, "out")) {
        Ok(bytes) => (0, bytes, String::new()),
        Err(e) => (2, Vec::new(), format!("{e}\n")),
    }
}

fn cmd_convert(rest: &[String]) -> (i32, Vec<u8>, String) {
    let (positional, flags) = collect(rest);
    let Some(file) = positional.first() else {
        return (
            2,
            Vec::new(),
            format!("convert requires <file>\n\n{}", usage()),
        );
    };
    let Some(target) = flag_value(&flags, "to") else {
        return (
            2,
            Vec::new(),
            format!("convert requires --to <format>\n\n{}", usage()),
        );
    };
    let data = match read_file(file) {
        Ok(d) => d,
        Err(e) => return (2, Vec::new(), format!("{e}\n")),
    };
    let doc = match format::parse_document(&data, None) {
        Ok((_, doc)) => doc,
        Err(e) => return (2, Vec::new(), format!("{e}\n")),
    };
    let encoded: Vec<u8> = match target {
        "json" => match format::to_json(&doc) {
            Ok(s) => s.into_bytes(),
            Err(e) => return (2, Vec::new(), format!("{e}\n")),
        },
        "jsonld" => match format::to_jsonld(&doc) {
            Ok(s) => s.into_bytes(),
            Err(e) => return (2, Vec::new(), format!("{e}\n")),
        },
        "msgpack" => match format::to_msgpack(&doc) {
            Ok(bytes) => bytes,
            Err(e) => return (2, Vec::new(), format!("{e}\n")),
        },
        "proto" | "protobuf" => format::to_protobuf(&doc),
        other => {
            return (
                2,
                Vec::new(),
                format!("unsupported --to `{other}` (supported: msgpack, json, jsonld, proto)\n"),
            );
        }
    };
    match write_output(&encoded, flag_value(&flags, "out")) {
        Ok(bytes) => (0, bytes, String::new()),
        Err(e) => (2, Vec::new(), format!("{e}\n")),
    }
}

fn cmd_gltf_embed(rest: &[String]) -> (i32, Vec<u8>, String) {
    let (positional, flags) = collect(rest);
    if positional.len() < 2 {
        return (
            2,
            Vec::new(),
            format!("gltf-embed requires <doc.json> <scene.gltf>\n\n{}", usage()),
        );
    }
    let doc_data = match read_file(&positional[0]) {
        Ok(d) => d,
        Err(e) => return (2, Vec::new(), format!("{e}\n")),
    };
    let gltf_data = match read_file(&positional[1]) {
        Ok(d) => d,
        Err(e) => return (2, Vec::new(), format!("{e}\n")),
    };
    let doc = match format::parse_document(&doc_data, None) {
        Ok((_, doc)) => doc,
        Err(e) => return (2, Vec::new(), format!("{e}\n")),
    };
    let gltf_value = match serde_json::from_slice::<serde_json::Value>(&gltf_data) {
        Ok(v) => v,
        Err(e) => return (2, Vec::new(), format!("glTF parse error: {e}\n")),
    };
    match gltf::gltf_embed(&gltf_value, &doc) {
        Ok(embedded) => {
            let rendered = serde_json::to_string_pretty(&embedded).unwrap_or_default();
            match write_output(rendered.as_bytes(), flag_value(&flags, "out")) {
                Ok(bytes) => (0, bytes, String::new()),
                Err(e) => (2, Vec::new(), format!("{e}\n")),
            }
        }
        Err(e) => (2, Vec::new(), format!("{e}\n")),
    }
}

fn cmd_gltf_extract(rest: &[String]) -> (i32, Vec<u8>, String) {
    let (positional, flags) = collect(rest);
    let Some(file) = positional.first() else {
        return (
            2,
            Vec::new(),
            format!("gltf-extract requires <scene.gltf>\n\n{}", usage()),
        );
    };
    let gltf_data = match read_file(file) {
        Ok(d) => d,
        Err(e) => return (2, Vec::new(), format!("{e}\n")),
    };
    let gltf_value = match serde_json::from_slice::<serde_json::Value>(&gltf_data) {
        Ok(v) => v,
        Err(e) => return (2, Vec::new(), format!("glTF parse error: {e}\n")),
    };
    match gltf::gltf_extract(&gltf_value) {
        Ok(doc) => {
            let rendered = match format::to_json(&doc) {
                Ok(s) => s,
                Err(e) => return (2, Vec::new(), format!("{e}\n")),
            };
            match write_output(rendered.as_bytes(), flag_value(&flags, "out")) {
                Ok(bytes) => (0, bytes, String::new()),
                Err(e) => (2, Vec::new(), format!("{e}\n")),
            }
        }
        Err(e) => (2, Vec::new(), format!("{e}\n")),
    }
}
