use vergen_git2::{Emitter, Git2Builder};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // set VERGEN_GIT_SHA env variable based on git commit hash
    let git2 = Git2Builder::default().branch(true).sha(true).build()?;
    Emitter::default().add_instructions(&git2)?.emit()?;

    let mut config = prost_build::Config::new();
    config.protoc_arg("--experimental_allow_proto3_optional");
    config.type_attribute(
        "license.LicenseLimits",
        "#[derive(serde::Serialize, serde::Deserialize)]",
    );
    tonic_build::configure().compile_with_config(
        config,
        &[
            "proto/core/auth.proto",
            "proto/core/proxy.proto",
            "proto/core/vpn.proto",
            "src/enterprise/proto/license.proto",
            "proto/worker/worker.proto",
            "proto/wireguard/gateway.proto",
        ],
        &[
            "proto/core",
            "proto/worker",
            "proto/wireguard",
            "src/enterprise/proto",
        ],
    )?;
    println!("cargo:rerun-if-changed=migrations");
    println!("cargo:rerun-if-changed=proto");
    println!("cargo:rerun-if-changed=web/dist");
    Ok(())
}
