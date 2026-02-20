# Optional if applicable for project
-include .env
export

# ==================================================================================== #
# HELPERS
# ==================================================================================== #

## help: print this help message
.PHONY: help
help:
	@echo 'Usage:'
	@sed -n 's/^##//p' ${MAKEFILE_LIST} | column -t -s ':' |  sed -e 's/^/ /'

.PHONY: confirm
confirm:
	@echo -n 'Are you sure? [y/N] ' && read ans && [ $${ans:-N} = y ]

# ==================================================================================== #
# DEVELOPMENT
# ==================================================================================== #

## dev: run the TUI app
.PHONY: dev

dev:
	@cargo run -p kubetile-app

## fmt: format code
.PHONY: fmt
fmt:
	@cargo fmt

## fmt-check: verify formatting
.PHONY: fmt-check
fmt-check:
	@cargo fmt --all -- --check

# ==================================================================================== #
# QUALITY CONTROL
# ==================================================================================== #

## lint: run clippy on the workspace
.PHONY: lint
lint:
	@cargo clippy --all-targets -- -D warnings

## test: run all tests
.PHONY: test
test:
	@cargo test --all

## test-core: run kubetile-core tests only
.PHONY: test-core
test-core:
	@cargo test -p kubetile-core

## coverage: show test coverage in terminal (requires cargo-llvm-cov)
.PHONY: coverage
coverage:
	@command -v cargo-llvm-cov >/dev/null 2>&1 || cargo install cargo-llvm-cov
	@cargo llvm-cov --workspace --summary-only

## tools: install developer tools used by this repo
.PHONY: tools
tools:
	@command -v cargo-llvm-cov >/dev/null 2>&1 || cargo install cargo-llvm-cov

# ==================================================================================== #
# BUILD
# ==================================================================================== #

## check: type-check without building
.PHONY: check
check:
	@cargo check --workspace

## build: build debug artifacts
.PHONY: build
build:
	@cargo build --workspace

## build-release: build release artifacts
.PHONY: build-release
build-release:
	@cargo build --release
