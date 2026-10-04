/// Embedded version.dll binary compiled from crates/proxy
pub const VERSION_DLL_BYTES: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../target/x86_64-pc-windows-gnu/release/version.dll"
));
