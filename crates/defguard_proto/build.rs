use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    tonic_prost_build::configure()
        // These types contain sensitive data.
        .skip_debug([
            "ActivateUserRequest",
            "AuthInfoResponse",
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
        .compile_protos(
            &[
                "../../proto/v1/worker/worker.proto",
                "../../proto/v2/common.proto",
                "../../proto/v2/proxy.proto",
                "../../proto/v2/gateway.proto",
                "../../proto/enterprise/v2/firewall/firewall.proto",
                "../../proto/common/client_types.proto",
            ],
            &["../../proto"],
        )?;

    println!("cargo:rerun-if-changed=../../proto");
    Ok(())
}
