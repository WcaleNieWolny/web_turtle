fn main() {
    println!("FUCK THIS HELL, {}", cfg!(target_arch = "wasm32"));
    if cfg!(target_arch = "wasm32") {
        println!("FUCK THIS HELL");
        println!(r#"cargo:rustc-cfg=feature="{}""#, "wasm");
    }
}
