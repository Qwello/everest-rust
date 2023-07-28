use anyhow::Result;
use argh::FromArgs;
use everest_build::schema::{DataTypes, Interface, Manifest};
use serde::de::DeserializeOwned;
use std::fs;
use std::path::PathBuf;

#[derive(FromArgs, PartialEq, Debug)]
/// Validate everest-core YAML files by reading them through a strongly typed system.
struct Args {
    #[argh(subcommand)]
    cmd: SubCommand,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand)]
enum SubCommand {
    Manifest(ManifestArgs),
    Interface(InterfaceArgs),
    DataTypes(DataTypesArgs),
}

#[derive(FromArgs, PartialEq, Debug)]
/// Validate a module manifest.yaml.
#[argh(subcommand, name = "manifest")]
struct ManifestArgs {
    /// manifest to parse
    #[argh(positional)]
    pub yaml: Vec<PathBuf>,
}

#[derive(FromArgs, PartialEq, Debug)]
/// Validate a type yaml.
#[argh(subcommand, name = "type")]
struct DataTypesArgs {
    /// manifest to parse
    #[argh(positional)]
    pub yaml: Vec<PathBuf>,
}

#[derive(FromArgs, PartialEq, Debug)]
/// Validate a module manifest.yaml.
#[argh(subcommand, name = "interface")]
struct InterfaceArgs {
    /// manifest to parse
    #[argh(positional)]
    pub yaml: Vec<PathBuf>,
}

fn validate<T: DeserializeOwned + std::fmt::Debug>(paths: &[PathBuf]) -> Result<()> {
    for p in paths {
        println!("Validating: {:#?}", p);
        let blob = fs::read_to_string(p)?;
        let _manifest: T = serde_yaml::from_str(&blob)?;
    }
    Ok(())
}

fn main() -> Result<()> {
    let args: Args = argh::from_env();
    match args.cmd {
        SubCommand::Manifest(args) => validate::<Manifest>(&args.yaml)?,
        SubCommand::Interface(args) => validate::<Interface>(&args.yaml)?,
        SubCommand::DataTypes(args) => validate::<DataTypes>(&args.yaml)?,
    }
    Ok(())
}
