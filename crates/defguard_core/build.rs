use vergen_git2::{Emitter, Git2Builder};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // set VERGEN_GIT_SHA env variable based on git commit hash
    let git2 = Git2Builder::default().branch(true).sha(true).build()?;
    Emitter::default().add_instructions(&git2)?.emit()?;

    tonic_prost_build::configure()
        // These types contain sensitive data.
        .skip_debug([
            "ActivateUserRequest",
            "AuthInfoResponse",
            "AuthenticateRequest",
            "AuthenticateResponse",
            "ClientMfaFinishResponse",
            "CodeMfaSetupStartResponse",
            "CodeMfaSetupFinishResponse",
            "CoreRequest",
            "CoreResponse",
            "DeviceConfigResponse",
            "InstanceInfoResponse",
            "NewDevice",
            "PasswordResetRequest",
        ])
        .protoc_arg("--experimental_allow_proto3_optional")
        .type_attribute(
            "LicenseLimits",
            "#[derive(serde::Serialize, serde::Deserialize)]",
        )
        .compile_protos(
            &[
                "../../proto/core/auth.proto",
                "../../proto/core/proxy.proto",
                "../../proto/worker/worker.proto",
                "../../proto/wireguard/gateway.proto",
                "../../proto/enterprise/firewall/firewall.proto",
                "src/enterprise/proto/license.proto",
            ],
            &[
                "../../proto/core",
                "../../proto/worker",
                "../../proto/wireguard",
                "../../proto/enterprise/firewall",
                "src/enterprise/proto",
            ],
        )?;
    println!("cargo:rerun-if-changed=../../proto");
    println!("cargo:rerun-if-changed=src/enterprise");
    Ok(())
}
