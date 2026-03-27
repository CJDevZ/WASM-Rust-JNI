fn main() {
    prost_build::Config::new()
        .compile_protos(
            &["src/bindings/universe.proto"],
            &["src/"]
        ).unwrap();
    println!("Protobufs compiled successfully!");
}
