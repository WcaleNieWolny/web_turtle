frontend:
	cargo +nightly build -Z build-std=std,panic_abort -Z build-std-features=panic_immediate_abort --target wasm32-unknown-unknown --release
	wasm-snip ../target/wasm32-unknown-unknown/release/bevy_frontend.wasm --snip-rust-panicking-code  --snip-rust-fmt-code -o $(realpath ./dist/output.wasm)
	wasm-opt -Oz -ol 100 --dce -s 100 -o $(realpath ./dist/output.wasm) $(realpath ./dist/output.wasm)
	# wasm-strip $(realpath ./dist/output.wasm)
	#
	#

DIST_PATH=$(shell realpath ./dist/)
FINAL_BIN_PATH=$(shell realpath ./dist/bevy_frontend_bg.wasm)

wasm-pack:
	rustup run nightly wasm-pack build --target web -d $(DIST_PATH) . -Z build-std=std,panic_abort -Z build-std-features=panic_immediate_abort
	wasm-snip $(FINAL_BIN_PATH) --snip-rust-panicking-code -o $(FINAL_BIN_PATH)
	wasm-opt -O4 -ol 100 --dce -s 100 -o $(FINAL_BIN_PATH) $(FINAL_BIN_PATH)
	cp -f ./index.html	$(DIST_PATH)
