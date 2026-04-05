## Rust-MC
### Example plugin written in Rust

Building the example plugin:\
``cargo build --target wasm32-wasip1 --release``\
``wasm-tools component new .\target\wasm32-wasip1\release\Rust_MC.wasm -o component.wasm --adapt wasi_snapshot_preview1.reactor.wasm``
