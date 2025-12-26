use std::env;
use std::path::PathBuf;

fn main() {
    // 1. Build the Swift package
    swift_rs::SwiftLinker::new("14.0")
        .with_package("yaad_swift", "src/yaad_swift")
        .link();
    
    // 2. Link the static library path explicitly
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let lib_path = out_dir.join("swift-rs/yaad_swift/arm64-apple-macosx/debug/libyaad_swift.a");
    
    if lib_path.exists() {
        println!("cargo:rustc-link-arg={}", lib_path.display());
    } else {
        let release_path = out_dir.join("swift-rs/yaad_swift/arm64-apple-macosx/release/libyaad_swift.a");
        if release_path.exists() {
            println!("cargo:rustc-link-arg={}", release_path.display());
        }
    }
    
    // 3. Link required macOS frameworks
    println!("cargo:rustc-link-lib=framework=AppKit");
    println!("cargo:rustc-link-lib=framework=Vision");
    println!("cargo:rustc-link-lib=framework=CoreGraphics");
    println!("cargo:rustc-link-lib=framework=CoreImage");
    println!("cargo:rustc-link-lib=framework=ScreenCaptureKit");
    
    // 4. CRITICAL FIX: Add system Swift libraries to the runtime search path
    // This resolves "Library not loaded: @rpath/libswift_Concurrency.dylib"
    println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib/swift");
    
    tauri_build::build()
}
