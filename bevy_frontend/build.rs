fn main() {
    if cfg!(target_arch = "wasm32") {
        println!(r#"cargo:rustc-cfg=feature="{}""#, "wasm");
    }
}
