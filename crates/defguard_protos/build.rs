fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = prost_build::Config::new();
    config.protoc_arg("--experimental_allow_proto3_optional");
    config.type_attribute(
        "license.LicenseLimits",
        "#[derive(serde::Serialize, serde::Deserialize)]",
    );
    tonic_build::configure().compile_protos_with_config(
        config,
        &[
            "proto/core/auth.proto",
            "proto/core/proxy.proto",
            "proto/worker/worker.proto",
            "proto/wireguard/gateway.proto",
            "proto/enterprise/firewall/firewall.proto",
        ],
        &[
            "proto/core",
            "proto/worker",
            "proto/wireguard",
            "proto/enterprise/firewall",
        ],
    )?;
    println!("cargo:rerun-if-changed=proto");
    Ok(())
}
