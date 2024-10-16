fn main() {
    tonic_build::configure()
        .build_server(false)
        .out_dir("src/client")
        .compile_protos(&["proto/disperser/disperser.proto"], &["proto"])
        .unwrap();
}
