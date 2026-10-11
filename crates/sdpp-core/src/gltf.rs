//! `EXT_spatial_dpp` embedding and extraction for glTF 2.0 (`.gltf`, JSON
//! container only in v0.1.0). See docs/gltf-extension/EXT_spatial_dpp.md.

use std::collections::HashMap;

use serde_json::{Map, Value};

use crate::model::{self, SpatialDppDocument};

pub const EXTENSION_NAME: &str = "EXT_spatial_dpp";

#[derive(Debug, thiserror::Error)]
pub enum GltfError {
    #[error("glTF root must be a JSON object")]
    NotAnObject,
    #[error("glTF document must contain a `nodes` array")]
    NoNodesArray,
    #[error("`nodes[{0}]` is not an object")]
    NodeNotObject(usize),
    #[error("object `{0}` has no glTF node with a matching `name`")]
    NoNodeForObject(String),
    #[error("glTF asset is missing `asset.extensions.EXT_spatial_dpp.metadata`")]
    NoMetadata,
    #[error("EXT_spatial_dpp metadata on the asset does not parse: {0}")]
    BadMetadata(String),
    #[error("EXT_spatial_dpp object on node {0} does not parse: {1}")]
    BadObject(usize, String),
    #[error("serialization error: {0}")]
    Serialize(String),
}

fn ensure_object<'a>(map: &'a mut Map<String, Value>, key: &str) -> &'a mut Map<String, Value> {
    let entry = map
        .entry(key.to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    if !entry.is_object() {
        *entry = Value::Object(Map::new());
    }
    entry
        .as_object_mut()
        .expect("value was just set to an object")
}

/// Embed a full document into a glTF 2.0 JSON scene:
/// - `asset.extensions.EXT_spatial_dpp.metadata` (once)
/// - `node.extensions.EXT_spatial_dpp.object` on the node whose `name`
///   equals each object's `id`
/// - `EXT_spatial_dpp` added to `extensionsUsed` (never `extensionsRequired`)
pub fn gltf_embed(gltf: &Value, doc: &SpatialDppDocument) -> Result<Value, GltfError> {
    let mut root = gltf.clone();
    if !root.is_object() {
        return Err(GltfError::NotAnObject);
    }
    let Some(nodes) = root.get("nodes").and_then(Value::as_array) else {
        return Err(GltfError::NoNodesArray);
    };

    // Resolve every placement up front so embedding fails before mutating.
    let mut placements = Vec::with_capacity(doc.objects.len());
    for object in &doc.objects {
        let index = nodes
            .iter()
            .position(|n| n.get("name").and_then(Value::as_str) == Some(object.id.as_str()));
        let Some(index) = index else {
            return Err(GltfError::NoNodeForObject(object.id.clone()));
        };
        placements.push((index, object.id.clone()));
    }

    let mut object_values: HashMap<String, Value> = HashMap::new();
    for object in &doc.objects {
        let value =
            serde_json::to_value(object).map_err(|e| GltfError::Serialize(e.to_string()))?;
        object_values.insert(object.id.clone(), value);
    }
    let metadata_value =
        serde_json::to_value(&doc.metadata).map_err(|e| GltfError::Serialize(e.to_string()))?;

    let root_map = root.as_object_mut().expect("checked above");

    match root_map
        .get_mut("extensionsUsed")
        .and_then(Value::as_array_mut)
    {
        Some(used) if used.iter().any(|v| v == EXTENSION_NAME) => {}
        Some(used) => used.push(Value::String(EXTENSION_NAME.to_string())),
        None => {
            root_map.insert(
                "extensionsUsed".into(),
                Value::Array(vec![Value::String(EXTENSION_NAME.to_string())]),
            );
        }
    }

    let asset = ensure_object(root_map, "asset");
    if asset.get("version").is_none() {
        asset.insert("version".into(), Value::String("2.0".into()));
    }
    let asset_extensions = ensure_object(asset, "extensions");
    let extension = ensure_object(asset_extensions, EXTENSION_NAME);
    extension.insert("metadata".into(), metadata_value);

    let nodes = root_map
        .get_mut("nodes")
        .expect("nodes verified above")
        .as_array_mut()
        .expect("nodes verified above");
    for (index, id) in placements {
        let node = nodes[index]
            .as_object_mut()
            .ok_or(GltfError::NodeNotObject(index))?;
        let node_extensions = ensure_object(node, "extensions");
        let extension = ensure_object(node_extensions, EXTENSION_NAME);
        extension.insert(
            "object".into(),
            object_values.get(&id).expect("built above").clone(),
        );
    }
    Ok(root)
}

/// Extract a document from a glTF scene. A missing `geometry.locator` is
/// auto-filled with `#/nodes/<index>` (extension spec, Section 5).
pub fn gltf_extract(gltf: &Value) -> Result<SpatialDppDocument, GltfError> {
    if !gltf.is_object() {
        return Err(GltfError::NotAnObject);
    }
    let metadata_value = gltf
        .pointer("/asset/extensions/EXT_spatial_dpp/metadata")
        .ok_or(GltfError::NoMetadata)?;
    let metadata: model::DocumentMetadata = serde_json::from_value(metadata_value.clone())
        .map_err(|e| GltfError::BadMetadata(e.to_string()))?;
    let nodes = gltf
        .get("nodes")
        .and_then(Value::as_array)
        .ok_or(GltfError::NoNodesArray)?;
    let mut objects = Vec::new();
    for (index, node) in nodes.iter().enumerate() {
        let Some(object_value) = node.pointer("/extensions/EXT_spatial_dpp/object") else {
            continue;
        };
        let mut object_value = object_value.clone();
        if object_value.pointer("/geometry/locator").is_none() {
            if let Some(geometry) = object_value
                .get_mut("geometry")
                .and_then(Value::as_object_mut)
            {
                geometry.insert("locator".into(), Value::String(format!("#/nodes/{index}")));
            }
        }
        let object: model::SpatialObject = serde_json::from_value(object_value)
            .map_err(|e| GltfError::BadObject(index, e.to_string()))?;
        objects.push(object);
    }
    Ok(SpatialDppDocument { metadata, objects })
}
