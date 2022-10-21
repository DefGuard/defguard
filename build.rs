fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure().compile(
        &[
            "proto/core/auth.proto",
            "proto/core/vpn.proto",
            "proto/worker/worker.proto",
            "proto/wireguard/gateway.proto",
        ],
        &["proto/core", "proto/worker", "proto/wireguard"],
    )?;
    println!("cargo:rerun-if-changed=proto");
    println!("cargo:rerun-if-changed=migrations");
    Ok(())
}
