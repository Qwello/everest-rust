upload:
	rsync -cav manifest.yaml Cargo.lock Cargo.toml src splinter.local:/home/sirver/code/everest/workspace/everest-core/build/dist/libexec/everest/modules/RustKvs/

