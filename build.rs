fn main() {
    prost_build::Config::new()
        .compile_protos(
            &[
                "protobuf/core.proto",
                "protobuf/player.proto",
                "protobuf/universe.proto",
                "protobuf/plugin.proto",
                "protobuf/sync.proto",
            ],
            &["protobuf"]
        ).expect("Failed to compile protos");

    println!("cargo:rerun-if-changed=protobuf/core.proto");
    println!("cargo:rerun-if-changed=protobuf/universe.proto");
}
