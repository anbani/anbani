PYTHON ?= python3
TOOLS := packages/py/tools
JS := packages/js
RS := packages/rs
.PHONY: hooks check fmt clippy test gen check-generated test-js test-py test-textart test-rs conformance wasm bench
hooks:      ; prek install
fmt:        ; cd $(RS) && cargo fmt --all
check: check-generated test-js test-py test-textart test-rs conformance
clippy:     ; cd $(RS) && cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
test: test-rs
gen:        ; "$(PYTHON)" $(TOOLS)/gen.py
check-generated: ; "$(PYTHON)" $(TOOLS)/gen.py --check && "$(PYTHON)" $(TOOLS)/versions.py
test-js:    ; npm --prefix $(JS) run build --workspace=anbani && npm --prefix $(JS) test --workspace=anbani
test-py:    ; "$(PYTHON)" -m pytest -c packages/py/anbani/pyproject.toml packages/py/anbani/tests
test-textart: ; npm --prefix $(JS) run build --workspace=anbani-textart && npm --prefix $(JS) test --workspace=anbani-textart
test-rs:    ; cd $(RS) && cargo fmt --all --check && cargo clippy --workspace --all-targets --all-features --locked -- -D warnings && cargo test --workspace --all-features --locked && cargo build -p anbani-core --no-default-features --locked
conformance: ; "$(PYTHON)" $(TOOLS)/archive-smoke.py bindings --artifacts $(RS)/target/archive-smoke/bindings
wasm:       ; cd $(RS) && cargo build -p anbani-wasm --target wasm32-unknown-unknown --release --locked
bench:      ; cd $(RS) && cargo bench --workspace
