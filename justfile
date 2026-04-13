set shell := ["bash", "-lc"]

run:
    source ~/.cargo/env && cargo run

test:
    source ~/.cargo/env && cargo test

install-local:
    ./scripts/install-local.sh

uninstall-local:
    ./scripts/uninstall-local.sh
