PORT ?= 3939
VAULT_PATH ?= ~/Notidium
FRONTEND_DIR := frontend
NPM := npm
CARGO := cargo

.PHONY: help dev dev-release backend frontend setup build build-frontend clean generate-sdk install bump-patch bump-minor bump-major publish

help: ## Show this help message
	@echo "Available commands:"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}'

# Development commands
dev: ## Start backend and frontend dev servers
	@echo "Starting backend on port $(PORT) (vault: $(VAULT_PATH)) and frontend on port 5173..."
	@cd $(FRONTEND_DIR) && if [ ! -d node_modules ]; then $(NPM) ci; fi
	@trap 'echo "\nShutting down..."; kill 0' INT TERM EXIT; \
	($(CARGO) run -- serve $(VAULT_PATH) -p $(PORT)) & \
	(cd $(FRONTEND_DIR) && $(NPM) run dev) & \
	wait

dev-release: ## Start backend (release) and frontend dev servers
	@echo "Starting backend (release) on port $(PORT) (vault: $(VAULT_PATH)) and frontend on port 5173..."
	@cd $(FRONTEND_DIR) && if [ ! -d node_modules ]; then $(NPM) ci; fi
	@trap 'echo "\nShutting down..."; kill 0' INT TERM EXIT; \
	($(CARGO) run --release -- serve $(VAULT_PATH) -p $(PORT)) & \
	(cd $(FRONTEND_DIR) && $(NPM) run dev) & \
	wait

backend: ## Start backend only
	$(CARGO) run -- serve $(VAULT_PATH) -p $(PORT)

frontend: ## Start frontend only
	cd $(FRONTEND_DIR) && $(NPM) run dev

# SDK generation
generate-sdk: ## Generate TypeScript API client from OpenAPI spec
	@echo "Generating TypeScript API client from OpenAPI spec..."
	@echo "Make sure the backend is running on port $(PORT)..."
	@cd $(FRONTEND_DIR) && \
		rm -rf src/api/* && \
		npx openapi-typescript-codegen \
			--input http://localhost:$(PORT)/api/openapi.json \
			--output src/api \
			--client fetch \
			--useOptions && \
		echo "Done. API client generated in $(FRONTEND_DIR)/src/api/"

# Build commands
build-frontend: ## Build frontend only
	cd $(FRONTEND_DIR) && if [ ! -d node_modules ]; then $(NPM) ci; fi && $(NPM) run build

build: build-frontend ## Build frontend and backend for production
	$(CARGO) build --release

install: build-frontend ## Build and install notidium with bundled UI
	$(CARGO) install --path .
	@echo ""
	@echo "✓ Notidium installed! Run 'notidium serve <vault-path>' to start."

clean: ## Clean build artifacts
	$(CARGO) clean
	cd $(FRONTEND_DIR) && rm -rf dist node_modules

# Setup commands
setup: ## Initial setup for new developers
	@echo "Setting up Notidium development environment..."
	@cd $(FRONTEND_DIR) && $(NPM) ci
	@echo "✓ Frontend dependencies installed"
	@echo ""
	@echo "Setup complete! Run 'make dev' to start development servers."
	@echo "Run 'make generate-sdk' (with backend running) to generate the API client."

# Version bumping
bump-patch: ## Bump patch version (0.1.0 -> 0.1.1)
	@VERSION=$$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/'); \
	MAJOR=$$(echo $$VERSION | cut -d. -f1); \
	MINOR=$$(echo $$VERSION | cut -d. -f2); \
	PATCH=$$(echo $$VERSION | cut -d. -f3); \
	NEW_PATCH=$$((PATCH + 1)); \
	NEW_VERSION="$$MAJOR.$$MINOR.$$NEW_PATCH"; \
	sed -i '' "s/^version = \"$$VERSION\"/version = \"$$NEW_VERSION\"/" Cargo.toml; \
	echo "Bumped version: $$VERSION -> $$NEW_VERSION"

bump-minor: ## Bump minor version (0.1.0 -> 0.2.0)
	@VERSION=$$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/'); \
	MAJOR=$$(echo $$VERSION | cut -d. -f1); \
	MINOR=$$(echo $$VERSION | cut -d. -f2); \
	NEW_MINOR=$$((MINOR + 1)); \
	NEW_VERSION="$$MAJOR.$$NEW_MINOR.0"; \
	sed -i '' "s/^version = \"$$VERSION\"/version = \"$$NEW_VERSION\"/" Cargo.toml; \
	echo "Bumped version: $$VERSION -> $$NEW_VERSION"

bump-major: ## Bump major version (0.1.0 -> 1.0.0)
	@VERSION=$$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/'); \
	MAJOR=$$(echo $$VERSION | cut -d. -f1); \
	NEW_MAJOR=$$((MAJOR + 1)); \
	NEW_VERSION="$$NEW_MAJOR.0.0"; \
	sed -i '' "s/^version = \"$$VERSION\"/version = \"$$NEW_VERSION\"/" Cargo.toml; \
	echo "Bumped version: $$VERSION -> $$NEW_VERSION"

# Publishing
publish: build-frontend ## Build and publish to crates.io
	@echo "Building frontend..."
	@echo "Checking package..."
	$(CARGO) package --list
	@echo ""
	@read -p "Ready to publish to crates.io. Continue? [y/N] " confirm; \
	if [ "$$confirm" = "y" ] || [ "$$confirm" = "Y" ]; then \
		$(CARGO) publish; \
		git add -A && git commit -m "Release v$$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')"; \
		git push; \
		echo "✓ Published to crates.io!"; \
	else \
		echo "Cancelled."; \
	fi
