use anyhow::Result;

mod codegen;
pub mod schema;

use std::io::Write;
use std::path::PathBuf;

#[derive(Debug, Default)]
pub struct Builder {
    everest_core: PathBuf,
    // TODO(sirver): This is almost always the same anyways.
    manifest_path: PathBuf,
    module_name: String,
    out_dir: Option<PathBuf>,
}

impl Builder {
    pub fn new(
        module_name: impl Into<String>,
        manifest_path: impl Into<PathBuf>,
        everest_core: impl Into<PathBuf>,
    ) -> Self {
        Self {
            everest_core: everest_core.into(),
            module_name: module_name.into(),
            manifest_path: manifest_path.into(),
            ..Builder::default()
        }
    }

    pub fn out_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.out_dir = Some(path.into());
        self
    }

    pub fn generate(self) -> Result<()> {
        let path = self
            .out_dir
            .unwrap_or_else(|| PathBuf::from(std::env::var("OUT_DIR").unwrap()))
            .join("generated.rs");

        let out = codegen::emit(self.module_name, self.manifest_path, self.everest_core)?;

        let mut f = std::fs::File::create(path)?;
        f.write_all(out.as_bytes())?;
        Ok(())
    }
}
