use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub description: String,
    pub provides: BTreeMap<String, ProvidesEntry>,
    pub metadata: Metadata,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct YamlData {
    pub description: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProvidesEntry {
    pub interface: String,
    pub description: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Metadata {
    pub license: String,
    pub authors: Vec<String>,
}
