fn main() {
    let mut config = prost_build::Config::new();
    config.bytes(&["."]);
    // config.type_attribute(".", "#[derive(PartialOrd)]");
    config
        .out_dir("src/proto")
        .compile_protos(&["abi.proto"], &["./src/proto"])
        .unwrap();
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=abi.proto");
}
