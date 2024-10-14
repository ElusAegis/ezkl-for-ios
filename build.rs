#[cfg(feature = "ios-bindings-build")]
use {
    camino::Utf8Path,
    std::path::{Path, PathBuf},
    std::fs,
    std::process::Command,
    uniffi_bindgen::bindings::SwiftBindingGenerator,
    uniffi_bindgen::library_mode::generate_bindings,
    uuid::Uuid,
};

fn main() {

    #[allow(unused_variables)]
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap();
    #[allow(unused_variables)]
    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap();

    #[cfg(not(feature = "wasm32-bindings"))]
    if target_arch == "wasm32" {
        panic!("Invalid feature configuration. Make sure to pass `wasm32-bindings` feature if building for WebAssembly.");
    }

    #[cfg(not(feature = "wasm32-unknown-bindings"))]
    if target_arch == "wasm32" && target_os == "unknown" {
        panic!("Invalid feature configuration. Make sure to pass `wasm32-unknown-bindings` feature if building for WebAssembly Unknown.");
    }

    #[cfg(all(feature = "ios-bindings-test", feature = "ios-bindings-build"))]
    compile_error!("Invalid feature configuration. Use exclusively one of `ios-bindings-test` or `ios-bindings-build` features.");

    #[cfg(not(any(feature = "ios-bindings", feature = "wasm32-bindings", feature = "non-ios-wasm-bindings")))]
    compile_error!("Invalid feature configuration. Make sure to pass `non-ios-wasm-bindings` feature if not building for iOS or WebAssembly.");

    if cfg!(feature = "ios-bindings-test") {
        // Set the environment variable for test builds
        println!("cargo:rustc-env=UNIFFI_CARGO_BUILD_EXTRA_ARGS=--features=ios-bindings --no-default-features");
    }
    #[cfg(feature = "ios-bindings-build")]
     {
        let mode = determine_build_mode();
        let library_name = std::env::var("CARGO_PKG_NAME").expect("CARGO_PKG_NAME is not set");
        build_bindings(&library_name, mode);
    }

    println!("cargo:rerun-if-changed=build.rs");

}

/// Determines the build mode based on the CONFIGURATION environment variable.
/// Defaults to "release" if not set or unrecognized.
/// "release" mode takes longer to build but produces optimized code, which has smaller size and is faster.
#[cfg(feature = "ios-bindings-build")]
fn determine_build_mode() -> &'static str {
    match std::env::var("CONFIGURATION").map(|s| s.to_lowercase()) {
        Ok(ref config) if config == "debug" => "debug",
        _ => "release",
    }
}

/// Builds the Swift bindings and XCFramework for the specified library and build mode.
#[cfg(feature = "ios-bindings-build")]
fn build_bindings(library_name: &str, mode: &str) {
    // Get the root directory of this Cargo project
    let manifest_dir = std::env::var_os("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap());

    // Define the build directory inside the manifest directory
    let build_dir = manifest_dir.join("build");

    // Create a temporary directory to store the bindings and combined library
    let tmp_dir = mktemp_local(&build_dir);

    // Define directories for Swift bindings and output bindings
    let swift_bindings_dir = tmp_dir.join("SwiftBindings");
    let bindings_out = create_bindings_out_dir(&tmp_dir);
    let framework_out = bindings_out.join("EzklCore.xcframework");

    // Define target architectures for building
    #[allow(clippy::useless_vec)]
    let target_archs = vec![
        vec!["aarch64-apple-ios"],       // iOS device
        // vec!["aarch64-apple-ios-sim"],   // iOS simulator ARM Mac
        // vec!["aarch64-apple-ios-sim", "x86_64-apple-ios"], // Uncomment to support older Macs (Intel)
    ];

    // Build the library for each architecture and combine them
    let out_lib_paths: Vec<PathBuf> = target_archs
        .iter()
        .map(|archs| build_combined_archs(library_name, archs, &build_dir, mode))
        .collect();

    // Generate the path to the built dynamic library (.dylib)
    let out_dylib_path = build_dir.join(format!(
        "{}/{}/lib{}.dylib",
        target_archs[0][0], mode, library_name
    ));

    // Generate Swift bindings using uniffi_bindgen
    generate_ios_bindings(&out_dylib_path, &swift_bindings_dir)
        .expect("Failed to generate iOS bindings");

    // Move the generated Swift file to the bindings output directory
    fs::rename(
        swift_bindings_dir.join(format!("{}.swift", library_name)),
        bindings_out.join("EzklCore.swift"),
    )
    .expect("Failed to copy swift bindings file");

    // Rename the `ios_ezklFFI.modulemap` file to `module.modulemap`
    fs::rename(
        swift_bindings_dir.join(format!("{}FFI.modulemap", library_name)),
        swift_bindings_dir.join("module.modulemap"),
    )
    .expect("Failed to rename modulemap file");

    // Create the XCFramework from the combined libraries and Swift bindings
    create_xcframework(&out_lib_paths, &swift_bindings_dir, &framework_out);

    // Define the destination directory for the bindings
    let bindings_dest = build_dir.join("EzklCoreBindings");
    if bindings_dest.exists() {
        fs::remove_dir_all(&bindings_dest).expect("Failed to remove existing bindings directory");
    }

    // Move the bindings output to the destination directory
    fs::rename(&bindings_out, &bindings_dest).expect("Failed to move framework into place");

    // Clean up temporary directories
    cleanup_temp_dirs(&build_dir);
}

/// Creates the output directory for the bindings.
/// Returns the path to the bindings output directory.
#[cfg(feature = "ios-bindings-build")]
fn create_bindings_out_dir(base_dir: &Path) -> PathBuf {
    let bindings_out = base_dir.join("EzklCoreBindings");
    fs::create_dir_all(&bindings_out).expect("Failed to create bindings output directory");
    bindings_out
}

/// Builds the library for each architecture and combines them into a single library using lipo.
/// Returns the path to the combined library.
#[cfg(feature = "ios-bindings-build")]
fn build_combined_archs(
    library_name: &str,
    archs: &[&str],
    build_dir: &Path,
    mode: &str,
) -> PathBuf {
    // Build the library for each architecture
    let out_lib_paths: Vec<PathBuf> = archs
        .iter()
        .map(|&arch| {
            build_for_arch(arch, build_dir, mode);
            build_dir
                .join(arch)
                .join(mode)
                .join(format!("lib{}.a", library_name))
        })
        .collect();

    // Create a unique temporary directory for the combined library
    let lib_out = mktemp_local(build_dir).join(format!("lib{}.a", library_name));

    // Combine the libraries using lipo
    let mut lipo_cmd = Command::new("lipo");
    lipo_cmd
        .arg("-create")
        .arg("-output")
        .arg(lib_out.to_str().unwrap());
    for lib_path in &out_lib_paths {
        lipo_cmd.arg(lib_path.to_str().unwrap());
    }

    let status = lipo_cmd.status().expect("Failed to run lipo command");
    if !status.success() {
        panic!("lipo command failed with status: {}", status);
    }

    lib_out
}

/// Builds the library for a specific architecture.
#[cfg(feature = "ios-bindings-build")]
fn build_for_arch(arch: &str, build_dir: &Path, mode: &str) {
    // Ensure the target architecture is installed
    install_arch(arch);

    // Run cargo build for the specified architecture and mode
    let mut build_cmd = Command::new("cargo");
    build_cmd
        .arg("build")
        .arg("--no-default-features")
        .arg("--features=ios-bindings");

    if mode == "release" {
        build_cmd.arg("--release");
    }
    build_cmd
        .arg("--lib")
        .env("CARGO_BUILD_TARGET_DIR", build_dir)
        .env("CARGO_BUILD_TARGET", arch);

    let status = build_cmd.status().expect("Failed to run cargo build");
    if !status.success() {
        panic!("cargo build failed for architecture: {}", arch);
    }
}

/// Installs the specified target architecture using rustup.
#[cfg(feature = "ios-bindings-build")]
fn install_arch(arch: &str) {
    let status = Command::new("rustup")
        .arg("target")
        .arg("add")
        .arg(arch)
        .status()
        .expect("Failed to run rustup command");

    if !status.success() {
        panic!("Failed to install target architecture: {}", arch);
    }
}

/// Generates Swift bindings for the iOS library using uniffi_bindgen.
#[cfg(feature = "ios-bindings-build")]
fn generate_ios_bindings(dylib_path: &Path, binding_dir: &Path) -> Result<(), std::io::Error> {
    // Remove existing binding directory if it exists
    if binding_dir.exists() {
        fs::remove_dir_all(binding_dir)?;
    }

    // Generate the Swift bindings using uniffi_bindgen
    generate_bindings(
        Utf8Path::from_path(dylib_path).ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid dylib path")
        })?,
        None,
        &SwiftBindingGenerator,
        None,
        Utf8Path::from_path(binding_dir).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid Swift bindings directory",
            )
        })?,
        true,
    )
    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

    Ok(())
}

/// Creates an XCFramework from the combined libraries and Swift bindings.
#[cfg(feature = "ios-bindings-build")]
fn create_xcframework(lib_paths: &[PathBuf], swift_bindings_dir: &Path, framework_out: &Path) {
    let mut xcbuild_cmd = Command::new("xcodebuild");
    xcbuild_cmd.arg("-create-xcframework");

    // Add each library and its corresponding headers to the xcodebuild command
    for lib_path in lib_paths {
        println!("Including library: {:?}", lib_path);
        xcbuild_cmd.arg("-library");
        xcbuild_cmd.arg(lib_path.to_str().unwrap());
        xcbuild_cmd.arg("-headers");
        xcbuild_cmd.arg(swift_bindings_dir.to_str().unwrap());
    }

    xcbuild_cmd.arg("-output");
    xcbuild_cmd.arg(framework_out.to_str().unwrap());

    let status = xcbuild_cmd.status().expect("Failed to run xcodebuild");
    if !status.success() {
        panic!("xcodebuild failed with status: {}", status);
    }
}

/// Creates a temporary directory inside the build path with a unique UUID.
/// This ensures unique build artifacts for concurrent builds.
#[cfg(feature = "ios-bindings-build")]
fn mktemp_local(build_path: &Path) -> PathBuf {
    let dir = tmp_local(build_path).join(Uuid::new_v4().to_string());
    fs::create_dir(&dir).expect("Failed to create temporary directory");
    dir
}

/// Gets the path to the local temporary directory inside the build path.
#[cfg(feature = "ios-bindings-build")]
fn tmp_local(build_path: &Path) -> PathBuf {
    let tmp_path = build_path.join("tmp");
    if let Ok(metadata) = fs::metadata(&tmp_path) {
        if !metadata.is_dir() {
            panic!("Expected 'tmp' to be a directory");
        }
    } else {
        fs::create_dir_all(&tmp_path).expect("Failed to create local temporary directory");
    }
    tmp_path
}

/// Cleans up temporary directories inside the build path.
#[cfg(feature = "ios-bindings-build")]
fn cleanup_temp_dirs(build_dir: &Path) {
    let tmp_dir = build_dir.join("tmp");
    if tmp_dir.exists() {
        fs::remove_dir_all(tmp_dir).expect("Failed to remove temporary directories");
    }
}