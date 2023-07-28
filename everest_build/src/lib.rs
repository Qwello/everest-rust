// NOCOM(#sirver): what
#![allow(unused)]
use anyhow::Result;

pub mod schema;

use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct Builder {
    everest_core_path: PathBuf,
}

impl Builder {
    pub fn new(everest_core_path: PathBuf) -> Self {
        Self { everest_core_path }
    }

    pub fn for_manifest(path: &Path) -> Result<()> {
        let manifest: schema::manifest::Manifest = {
            let blob = fs::read_to_string(path)?;
            serde_yaml::from_str(&blob)?
        };

        println!("#sirver manifest: {:#?}", manifest);
        Ok(())
    }
}
