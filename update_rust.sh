#!/bin/bash
if [[ -z "$SKIP_RUST_UPDATES" ]]; then
 if git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  git fetch
  git stash --include-untracked
  git pull
  git stash pop
 fi

 rustup self update
 rustup update

 cargo update
 cargo generate-lockfile

 cargo install cargo-binstall

 cargo binstall -y --force cargo-binstall
 cargo binstall -y cross

 cargo binstall -y --force cargo-update
 cargo install-update --git --all

 # shellcheck disable=SC2086
 curl -LsSf https://get.nexte.st/latest/linux | tar zxf - -C ${CARGO_HOME:-~/.cargo}/bin

 if command -v podman >/dev/null 2>&1; then
  podman image ls --format "{{.Repository}}:{{.Tag}}" | while read -r container ; do
   podman pull "$container"
  done
  podman image prune -f
 fi

 cargo binstall -y cargo-sweep

 # Prevent target folder from growing to a gigantic size
 cargo sweep --toolchains nightly-x86_64-unknown-linux-gnu

 # Remove old versions of crates installed by cargo-install
 rm -f ~/.cargo/bin/*-v*
fi

