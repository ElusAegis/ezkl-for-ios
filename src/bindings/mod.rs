/// Universal bindings for all platforms
pub mod universal;
/// wasm prover and verifier
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
pub mod wasm;