fn main() {
    // `screencapturekit` (pulled in by ruhear for system-audio "capture all") links a
    // Swift shim that depends on the Swift concurrency runtime. On macOS those dylibs
    // live in the dyld shared cache under /usr/lib/swift, which isn't on the default
    // search path for a Rust-linked binary — so add it as an rpath, otherwise the app
    // aborts at launch with "Library not loaded: @rpath/libswift_Concurrency.dylib".
    #[cfg(target_os = "macos")]
    {
        println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib/swift");
    }

    tauri_build::build()
}
