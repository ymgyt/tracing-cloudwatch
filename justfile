# Show available recipes by default.
default:
    @just --list --unsorted --list-submodules

check: lint test

cargo-hack-args := "--feature-powerset --include-features rusoto,rusoto_rustls,awssdk,ordered_logs --mutually-exclusive-features rusoto,rusoto_rustls --exclude-all-features"

lint:
    cargo fmt --check
    cargo sort --check
    cargo hack clippy {{cargo-hack-args}}

test:
    cargo hack test {{cargo-hack-args}}
