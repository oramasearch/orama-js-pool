use deno_web::BlobStore;
use std::env;
use std::path::PathBuf;
use std::sync::Arc;

#[path = "src/orama_extension.rs"]
mod orama_extension;
#[path = "src/permission.rs"]
mod permission;

deno_core::extension!(deno_telemetry, esm = ["telemetry.ts", "util.ts"],);

fn main() {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let snapshot_path = out_dir.join("RUNJS_SNAPSHOT.bin");

    let blob_store = BlobStore::default();
    let blob_store = Arc::new(blob_store);

    let snapshot = deno_core::snapshot::create_snapshot(
        deno_core::snapshot::CreateSnapshotOptions {
            cargo_manifest_dir: env!("CARGO_MANIFEST_DIR"),
            startup_snapshot: None,
            skip_op_registration: false,
            extensions: vec![
                deno_telemetry::init_ops_and_esm(),
                deno_webidl::deno_webidl::init_ops_and_esm(), // deno_url & deno_web
                deno_url::deno_url::init_ops_and_esm(),       // deno_web
                deno_console::deno_console::init_ops_and_esm(), // deno_web
                deno_web::deno_web::init_ops_and_esm::<permission::CustomPermissions>(
                    blob_store, None,
                ),
                deno_net::deno_net::init_ops_and_esm::<permission::CustomPermissions>(None, None), // deno_web
                deno_fetch::deno_fetch::init_ops_and_esm::<permission::CustomPermissions>(
                    deno_fetch::Options::default(),
                ),
                deno_crypto::deno_crypto::init_ops_and_esm(None), // deno_crypto
                orama_extension::orama_extension::init_ops_and_esm(
                    permission::CustomPermissions {
                        // During the snapshot build, no permissions are allowed
                        // NB: snapshot doesn't store the permissions, so this is just a dummy value
                        allowed_hosts: Some(vec![]),
                    },
                    orama_extension::ChannelStorage::<serde_json::Value> {
                        stream_handler: None,
                    },
                    orama_extension::StdoutHandler(None),
                    orama_extension::SharedCache::new(),
                ),
            ],
            with_runtime_cb: None,
            extension_transpiler: None,
        },
        None,
    )
    .unwrap();

    std::fs::write(snapshot_path, snapshot.output).unwrap();
}
