#!/bin/bash
export CLIPPY_ARGS="--all-targets --all-features -- -D warnings -W clippy::all -W clippy::style -W clippy::pedantic -W clippy::nursery -W clippy::perf -W clippy::suspicious -W clippy::cargo -W clippy::restriction -W clippy::deprecated -W clippy::exit -W clippy::dbg_macro -W clippy::unwrap_used -W clippy::complexity -W clippy::create_dir -W clippy::correctness -W clippy::expect_used -W clippy::too-many-lines -W clippy::must-use-candidate -W clippy::multiple-crate-versions"

if [[ -n "$GITHUB_ENV" ]]; then
 echo "CLIPPY_ARGS=$CLIPPY_ARGS" >> "$GITHUB_ENV"
fi
