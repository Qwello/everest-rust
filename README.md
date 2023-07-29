# Everest Rust implementation

This is a framework to build [EVerest](https://github.com/EVerest) in Rust. It
is meant to make it easy and convenient to build Nodes for EVerest in Rust. It
comes with the following crates:

- `everest_build` is the code generator that can take a `manifest.yaml` and turn
  it into interfaces and glue code for a Node. Normally, user code will have a
  `build-dependency` on this one.
- `everest` is the library containing the core logic & data types for EVerest.
  Normally, user code will have a `dependency` on this.
- `rust_kvs` is an example node. It implements a simple in memory store that
  provides the
  [`kvs`](https://github.com/EVerest/everest-core/blob/dfe28df90b38505faa724d980838c1e63d93fff4/interfaces/kvs.yaml)
  interface on the `main` slot.

## Trying it out

If you never worked with Rust, here is a quickstart:

- Install rust as outlined on <https://rustup.rs/>, which should just be this
  one line: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- Clone this repository, cd into `rust_kvs` and edit `build.rs` - adapt the path
  to `everest-core` there so that the YAMLs are found during codegen.
- Run `cargo build --release`. The binary will end up in
  `../target/release/rust_kvs`.
- Copy this binary and the `manifest.yaml` into a everest
  `libexec/everest/modules/RustKvs` directory.
- Change your runtime config to use `RustKvs` instead of `Store`.

## Goals

This project was started with the primary goal to understand how EVerest is
architected and how it works "under the hood". Hence it is actually a pure Rust
implementation, not relying on any of the EVerest tooling.

It is hence currently unofficial, since it uses EVerest implementation details
that are not guaranteed to be stable. Qwello wants to use this in production,
hence we want to work with the EVerest community to figure out how this can
become official.

## Status

This is a minimal viable implementation. It can currently parse and understand
all YAML files in `everest-core/[types,interfaces,modules]/**/manifest.yaml`. It
implements enough code gen and logic to build nodes that `provides` interfaces
and has no `requires`.

Missing are these a least. None of them are hard to implement, I started with
requires since that seemed the most difficult. They are just not done yet and I
wanted early feedback before continuing.

- Integration into EVerests build system
- Client codegen, i.e. `requires`.
- Support pub/sub of variables.
- support for objects and $ref.
- testing support that does not require MQTT running.

## Open questions

This are things I ran into that I do not understand yet:

- I saw protobuf mentioned in some documentation, but it (sadly) does not make
  an appearance. Everything seems to be JSON. Why is protobuf mentioned?
- How does a node know which connection it needs to listen to? Is it told by the
  framework or does it need to parse the configuration yaml (that it gets
  through `--conf` on the commandline) itself?
- Why does every note publish `metadata` with the slots it provides? Is this so
  the manager can valid that all configured connections in the runtime config
  are actually also available?
