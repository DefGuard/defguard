fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_prost_build::configure()
        .protoc_arg("--experimental_allow_proto3_optional")
        .type_attribute(
            "LicenseLimits",
            "#[derive(serde::Serialize, serde::Deserialize)]",
        )
        .compile_protos(
            &["src/enterprise/proto/license.proto"],
            &["src/enterprise/proto"],
        )?;
    println!("cargo:rerun-if-changed=src/enterprise/proto");
    Ok(())
}
