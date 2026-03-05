fn main() {
    // No linking needed for dynamic loading
    // We will load the DLL at runtime using libloading

    // Tell Cargo to rerun this build script (and recompile) whenever
    // any file inside the assets directory changes.
    println!("cargo:rerun-if-changed=assets/");
}
