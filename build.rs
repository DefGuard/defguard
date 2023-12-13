fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = prost_build::Config::new();
    config.protoc_arg("--experimental_allow_proto3_optional");
    tonic_build::configure().compile_with_config(
        config,
        &[
            "proto/core/auth.proto",
            "proto/core/vpn.proto",
            "proto/worker/worker.proto",
            "proto/wireguard/gateway.proto",
            "proto/enrollment/enrollment.proto",
            "proto/password_reset/password_reset.proto",
        ],
        &[
            "proto/core",
            "proto/worker",
            "proto/wireguard",
            "proto/enrollment",
            "proto/password_reset",
        ],
    )?;
    println!("cargo:rerun-if-changed=proto");
    println!("cargo:rerun-if-changed=migrations");
    Ok(())
}
