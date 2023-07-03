frontend:
	cargo +nightly build -Z build-std=std,panic_abort -Z build-std-features=panic_immediate_abort --target wasm32-unknown-unknown --release
	wasm-snip ../target/wasm32-unknown-unknown/release/bevy_frontend.wasm --snip-rust-panicking-code  --snip-rust-fmt-code -o $(realpath ./dist/output.wasm)
	wasm-opt -Oz -ol 100 --dce -s 100 -o $(realpath ./dist/output.wasm) $(realpath ./dist/output.wasm)
	# wasm-strip $(realpath ./dist/output.wasm)
	#
	#

RUSTC=$(shell rustup which --toolchain nightly rustc)
CARGO=$(shell rustup which --toolchain nightly cargo)

wasm-pack:
	rustup run nightly wasm-pack build --target web -d $(realpath ./dist/) . -Z build-std=std,panic_abort -Z build-std-features=panic_immediate_abort 
