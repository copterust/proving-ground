fn main() {
    println!("cargo:rustc-link-arg-bins=--nmagic");
    println!("cargo:rustc-link-arg-bins=-Tlink.x");
    if cfg!(feature = "with_defmt") {
        println!("cargo:rustc-link-arg-bins=-Tdefmt.x");
    }
}
