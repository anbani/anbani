//! Link setup for a Node addon (napi-build: undefined napi_* symbols resolve when Node loads it).

fn main() {
    napi_build::setup();
}
