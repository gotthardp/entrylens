# SPDX-License-Identifier: MPL-2.0
#
# Build, test and package the server; see README.md. Run `make help` for the targets. The
# backend and the frontend have their own Makefiles with more targets (`make -C backend help`).

# Backend configuration used by `make run` and `make dev`, relative to backend/.
CONFIG ?= dev.toml

.PHONY: help build test coverage lint format deb api demo run dev clean

help: ## Show the targets
	@grep -E '^[a-z-]+:.*## ' $(MAKEFILE_LIST) | awk -F ':.*## ' '{printf "  %-8s %s\n", $$1, $$2}'

build: ## Build the frontend and the release binary (which embeds it)
	$(MAKE) -C frontend build
	$(MAKE) -C backend release

test: ## Run the frontend's unit tests and the backend tests (which serve the built frontend)
	$(MAKE) -C frontend test
	$(MAKE) -C frontend build
	$(MAKE) -C backend test

coverage: ## Measure the backend's test coverage (needs the llvm package of rustc's version)
	$(MAKE) -C frontend build
	$(MAKE) -C backend coverage

lint: ## Run all checks: rustfmt, clippy, tsc, ESLint, Prettier
	$(MAKE) -C backend lint
	$(MAKE) -C frontend lint

format: ## Reformat the backend and the frontend
	$(MAKE) -C backend format
	$(MAKE) -C frontend format

deb: build ## Build the .deb package into backend/target/debian/
	deploy/build-deb.sh

api: ## Regenerate openapi.json and the frontend's types after an API change
	$(MAKE) -C backend openapi
	$(MAKE) -C frontend api

demo: ## Create backend/entrylens.db anew with the demo data (stop the server first)
	$(MAKE) -C frontend build
	$(MAKE) -C backend demo

run: ## Run only the backend from source (CONFIG, default dev.toml)
	$(MAKE) -C backend run CONFIG=$(CONFIG)

# The backend runs in the background; Ctrl+C stops both, and so does the development server
# exiting by itself.
dev: ## Run the backend and the frontend development server (reachable from the LAN)
	$(MAKE) -C backend build
	$(MAKE) -C frontend deps
	cd backend && ENTRYLENS_CONFIG=$(CONFIG) cargo run -q & pid=$$!; \
	trap 'kill $$pid 2>/dev/null' EXIT; \
	$(MAKE) -C frontend dev

clean: ## Remove all build output
	$(MAKE) -C backend clean
	$(MAKE) -C frontend clean
