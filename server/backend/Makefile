# SPDX-License-Identifier: MPL-2.0
#
# Backend commands; see ../README.md. Run `make help` for the targets.

# Configuration used by `make run`; e.g. `make run CONFIG=lan.toml`.
CONFIG ?= dev.toml
# Database used by `make demo`, as in dev.toml.
DATABASE ?= entrylens.db

# The coverage tools of rustc's LLVM version (Ubuntu package llvm-<version>).
LLVM_VERSION := $(shell rustc -vV | sed -n 's/^LLVM version: \([0-9]*\).*/\1/p')
LLVM_PROFDATA ?= llvm-profdata-$(LLVM_VERSION)
LLVM_COV ?= llvm-cov-$(LLVM_VERSION)
# The instrumented build, its profiles and temporary files go to target/coverage, not to /tmp,
# which may be a small tmpfs.
COVERAGE := $(CURDIR)/target/coverage
COVERAGE_ENV := RUSTFLAGS="-C instrument-coverage" CARGO_TARGET_DIR=$(COVERAGE)/target \
	LLVM_PROFILE_FILE=$(COVERAGE)/profiles/%p-%m.profraw TMPDIR=$(COVERAGE)/tmp

.PHONY: help build release run test coverage lint format openapi demo clean

help: ## Show the targets
	@grep -E '^[a-z-]+:.*## ' $(MAKEFILE_LIST) | awk -F ':.*## ' '{printf "  %-8s %s\n", $$1, $$2}'

build: ## Build the debug binary
	cargo build

release: ## Build the release binary
	cargo build --release

run: ## Run the server with CONFIG (default dev.toml)
	ENTRYLENS_CONFIG=$(CONFIG) cargo run

test: ## Run the tests (the frontend test needs ../frontend/dist)
	cargo test

coverage: ## Measure the test coverage; details in target/coverage/html/index.html
	rm -rf $(COVERAGE)/profiles
	mkdir -p $(COVERAGE)/tmp
	$(COVERAGE_ENV) cargo test --lib --tests
	$(LLVM_PROFDATA) merge -sparse $(COVERAGE)/profiles/*.profraw -o $(COVERAGE)/tests.profdata
	objects=$$($(COVERAGE_ENV) cargo test --lib --tests --no-run --message-format=json \
	        | grep -o '"executable":"[^"]*"' | cut -d'"' -f4 | sed 's/^/--object /'); \
	options="--instr-profile=$(COVERAGE)/tests.profdata --ignore-filename-regex=/.cargo/|/rustc[-/]|/tests/"; \
	$(LLVM_COV) report $$objects $$options && \
	$(LLVM_COV) show $$objects $$options --format=html --output-dir=$(COVERAGE)/html

lint: ## Check formatting and run clippy
	cargo fmt --check
	cargo clippy --all-targets -- -D warnings

format: ## Reformat with rustfmt
	cargo fmt

openapi: ## Regenerate openapi.json from the code
	UPDATE_OPENAPI=1 cargo test --test api openapi

# The server creates the tables (and records its migrations), so it runs briefly on a free port;
# it may be stopped before it handles signals, so its exit status is ignored, and the tables are
# checked instead.
demo: build ## Create DATABASE (default entrylens.db) anew with the demo data (stop the server first)
	rm -f $(DATABASE) $(DATABASE)-shm $(DATABASE)-wal
	printf 'listen = "127.0.0.1:0"\ndatabase = "%s"\n' "$(DATABASE)" > target/demo.toml
	ENTRYLENS_CONFIG=target/demo.toml target/debug/entrylens & pid=$$!; \
	for i in $$(seq 100); do \
	    sqlite3 $(DATABASE) "SELECT 1 FROM _sqlx_migrations" >/dev/null 2>&1 && break; \
	    sleep 0.2; \
	done; \
	kill $$pid; wait $$pid 2>/dev/null || true
	sqlite3 $(DATABASE) "SELECT 1 FROM _sqlx_migrations" >/dev/null
	sqlite3 $(DATABASE) < ../demo/layout.sql
	sqlite3 $(DATABASE) < ../demo/passages.sql
	sqlite3 $(DATABASE) < ../demo/check.sql

clean: ## Remove the build
	cargo clean
