set -xe

cmake --version || ( echo 'pls install cmake https://cmake.org/download/' && exit 1 )

cargo install --git https://github.com/DioxusLabs/dioxus dioxus-cli
cargo install wasm-bindgen-cli
