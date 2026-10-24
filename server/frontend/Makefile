# SPDX-License-Identifier: MPL-2.0
#
# Frontend commands; see ../README.md. Run `make help` for the targets.

.PHONY: help deps build dev test lint format api clean

help: ## Show the targets
	@grep -E '^[a-z-]+:.*## ' $(MAKEFILE_LIST) | awk -F ':.*## ' '{printf "  %-8s %s\n", $$1, $$2}'

# Reinstalled only when package-lock.json changes.
node_modules/.package-lock.json: package-lock.json
	npm ci

deps: node_modules/.package-lock.json ## Install the dependencies

build: deps ## Build into dist/ (served by the backend)
	npm run build

dev: deps ## Run the development server (reachable from the LAN)
	npm run dev

test: deps ## Run the unit tests
	npm test

lint: deps ## Check types, ESLint and Prettier
	npm run lint

format: deps ## Reformat with Prettier
	npm run format

api: deps ## Regenerate src/api/schema.d.ts from ../backend/openapi.json
	npm run gen:api

clean: ## Remove the build and the dependencies
	rm -rf dist node_modules
