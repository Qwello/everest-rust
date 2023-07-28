use everest_build::Builder;

fn main() {
    Builder::new("RustKvs", "manifest.yaml", "../../everest-core/")
        .generate()
        .unwrap();
}
