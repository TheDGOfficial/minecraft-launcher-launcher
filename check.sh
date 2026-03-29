#!/bin/bash
./format.sh
./test.sh
# shellcheck disable=SC2086
cargo +nightly clippy --fix --allow-dirty --allow-no-vcs $CLIPPY_ARGS
