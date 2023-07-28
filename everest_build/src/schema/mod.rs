use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;

pub mod interface;
pub mod manifest;

pub use interface::Interface;
pub use manifest::Manifest;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DataTypes {
    pub description: String,
    pub types: HashMap<String, interface::Variable>,
}
