//! Protobuf sources vendored from HashiCorp Terraform (official plugin protocol).
//!
//! - tfplugin5.proto — Terraform Plugin Protocol v5.10
//!   Source: https://github.com/hashicorp/terraform/blob/main/docs/plugin-protocol/tfplugin5.proto
//!
//! - tfplugin6.proto — Terraform Plugin Protocol v6.11
//!   Source: https://github.com/hashicorp/terraform/blob/main/docs/plugin-protocol/tfplugin6.proto
//!
//! Per HashiCorp guidance, prefer copying from a release tag for production use.
//! These files are from the `main` branch at integration time.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    std::env::set_var("PROTOC", protoc_bin_vendored::protoc_bin_path().unwrap());
    let protoc_include = protoc_bin_vendored::include_path().unwrap();
    let include_path = protoc_include.to_str().expect("protoc include path utf-8");

    tonic_build::configure()
        .build_server(false)
        .build_client(true)
        // Strip proto doc comments — HashiCorp warnings are not valid Rust doctests.
        .disable_comments(".")
        .compile_protos(
            &[
                "proto/provider.proto",
                "proto/tfplugin5.proto",
                "proto/tfplugin6.proto",
            ],
            &["proto/", include_path],
        )?;
    Ok(())
}
