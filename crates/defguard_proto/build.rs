use std::{
    env,
    error::Error,
    path::{Path, PathBuf},
};

const PROTO_LAYOUT_PATHS: [&str; 4] = [
    "v2/core/proxy.proto",
    "v2/worker/worker.proto",
    "v2/wireguard/gateway.proto",
    "enterprise/v2/firewall/firewall.proto",
];

fn main() -> Result<(), Box<dyn Error>> {
    let proto_repository_root = derive_proto_repository_root()?;
    let proto_files = resolve_proto_files(&proto_repository_root)?;

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
        .compile_protos(&proto_files, &[proto_repository_root.clone()])?;

    println!("cargo:rerun-if-changed={}", proto_repository_root.display());
    Ok(())
}

/// Derives the shared proto checkout from Cargo metadata so code generation does
/// not depend on the shell's current working directory.
fn derive_proto_repository_root() -> Result<PathBuf, Box<dyn Error>> {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let workspace_root = manifest_dir
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| {
            format!(
                "failed to derive workspace root from CARGO_MANIFEST_DIR: {}",
                manifest_dir.display()
            )
        })?;
    let proto_repository_root = workspace_root.join("proto");

    if !proto_repository_root.is_dir() {
        return Err(format!(
            "expected proto repository root at {}, but the directory does not exist",
            proto_repository_root.display()
        )
        .into());
    }

    Ok(proto_repository_root)
}

/// Resolves the expected protobuf entrypoints up front so missing definitions
/// fail the build before code generation starts.
fn resolve_proto_files(proto_repository_root: &Path) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    PROTO_LAYOUT_PATHS
        .iter()
        .map(|relative_path| {
            let proto_file = proto_repository_root.join(relative_path);
            if !proto_file.is_file() {
                return Err(format!(
                    "expected protobuf definition at {}, but the file does not exist",
                    proto_file.display()
                )
                .into());
            }

            Ok(proto_file)
        })
        .collect()
}
