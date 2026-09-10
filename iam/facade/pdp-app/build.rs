use std::error::Error;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn Error>> {
    let manifest_dir =
        PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string()));
    // Crate-local because buck2's genrule copies the proto to the same relative
    // path inside the generated manifest dir, so one resolution serves cargo and
    // buck2 alike.
    let proto_root = manifest_dir.join("proto");
    let proto_file = proto_root.join("iam-pdp.proto");
    if !proto_file.exists() {
        return Err(format!("missing proto contract {}", proto_file.display()).into());
    }

    println!("cargo:rerun-if-changed={}", proto_root.display());
    println!("cargo:rerun-if-changed={}", proto_file.display());

    let protoc = protoc_bin_vendored::protoc_bin_path()?;
    // SAFETY: Cargo build scripts execute single-threaded for this process
    // before invoking prost/tonic code generation. The PROTOC variable is
    // scoped to this build process and avoids dependence on mutable host CI
    // installations (the identity-workload-rest precedent).
    unsafe {
        std::env::set_var("PROTOC", protoc);
    }

    // Clients too, so the E2E suite drives the same generated contract the
    // server implements rather than a hand-rolled tonic call.
    tonic_prost_build::configure()
        .build_client(true)
        .build_server(true)
        .emit_rerun_if_changed(false)
        .compile_protos(&[&proto_file], &[&proto_root])?;

    Ok(())
}
