# Ethos - Decentralized Reputation System Makefile

# Variables
WASM_TARGET = wasm32-unknown-unknown
RUST_VERSION = 1.89.0

# Contract directories (individual projects, not workspace members)
CONTRACTS = issuer_registry sbt sbt_factory

# Colors for output
GREEN = \033[0;32m
YELLOW = \033[1;33m
RED = \033[0;31m
NC = \033[0m # No Color

# Load environment variables from .env.local if it exists
-include .env.local

# Default target
.PHONY: help
help:
	@echo "$(GREEN)Ethos - Decentralized Reputation System$(NC)"
	@echo "Available commands:"
	@echo "  $(YELLOW)make check$(NC)          - Check all contracts compile"
	@echo "  $(YELLOW)make build$(NC)          - Build all contracts to WASM"
	@echo "  $(YELLOW)make export-abi$(NC)     - Export all contract ABIs"
	@echo "  $(YELLOW)make clean$(NC)          - Clean all build artifacts"
	@echo "  $(YELLOW)make test$(NC)           - Run all tests"
	@echo "  $(YELLOW)make fmt$(NC)            - Format all code"
	@echo "  $(YELLOW)make clippy$(NC)         - Run clippy on all contracts"
	@echo "  $(YELLOW)make setup$(NC)          - Setup development environment"
	@echo ""
	@echo "Deployment commands:"
	@echo "  $(YELLOW)make deploy-devnet$(NC)       - Deploy all to devnet (localhost)"
	@echo "  $(YELLOW)make deploy-testnet$(NC)      - Deploy all to Arbitrum Sepolia"
	@echo "  $(YELLOW)make deploy-devnet-<contract>$(NC)   - Deploy specific contract to devnet"
	@echo "  $(YELLOW)make deploy-testnet-<contract>$(NC)  - Deploy specific contract to testnet"
	@echo ""
	@echo "Individual contract commands:"
	@echo "  $(YELLOW)make check-<contract>$(NC)     - Check specific contract"
	@echo "  $(YELLOW)make build-<contract>$(NC)     - Build specific contract"
	@echo "  $(YELLOW)make export-abi-<contract>$(NC) - Export specific contract ABI"
	@echo ""
	@echo "Available contracts: $(CONTRACTS)"
	@echo "Using Rust $(RUST_VERSION) and target $(WASM_TARGET)"


# Setup development environment
.PHONY: setup
setup: generate-lockfiles
	@echo "$(GREEN)Setting up development environment...$(NC)"
	@rustup toolchain install $(RUST_VERSION)
	@rustup target add $(WASM_TARGET) --toolchain $(RUST_VERSION)
	@rustup override set $(RUST_VERSION)
	@cargo install --force cargo-stylus
	@if [ ! -f .env.local ]; then \
		echo "$(YELLOW)Creating .env.local from template...$(NC)"; \
		cp .env.example .env.local; \
		echo "$(YELLOW)Please edit .env.local with your actual values$(NC)"; \
	fi
	@echo "$(GREEN)Setup complete!$(NC)"


# Check all contracts
.PHONY: check
check:
	@echo "$(GREEN)Checking all contracts...$(NC)"
	@for contract in $(CONTRACTS); do \
		echo "$(YELLOW)Checking $$contract...$(NC)"; \
		cd $$contract; \
		cargo stylus check; \
		if [ $$? -ne 0 ]; then \
			echo "$(RED)Failed to check $$contract$(NC)"; \
			cd ..; \
			exit 1; \
		fi; \
		cd ..; \
	done
	@echo "$(GREEN)All contracts check passed!$(NC)"


# Build all contracts
.PHONY: build
build:
	@echo "$(GREEN)Building all contracts...$(NC)"
	@for contract in $(CONTRACTS); do \
		echo "$(YELLOW)Building $$contract...$(NC)"; \
		cd $$contract && cargo build --target $(WASM_TARGET) --release && cd ..; \
		if [ $$? -ne 0 ]; then \
			echo "$(RED)Failed to build $$contract$(NC)"; \
			exit 1; \
		fi; \
	done
	@echo "$(GREEN)All contracts built successfully!$(NC)"


# Export all ABIs
.PHONY: export-abi
export-abi:
	@echo "$(GREEN)Exporting all contract ABIs...$(NC)"
	@mkdir -p abis
	@for contract in $(CONTRACTS); do \
		echo "$(YELLOW)Exporting ABI for $$contract...$(NC)"; \
		cd $$contract && cargo stylus export-abi --json > ../abis/$$contract.json && cd ..; \
		if [ $$? -ne 0 ]; then \
			echo "$(RED)Failed to export ABI for $$contract$(NC)"; \
			exit 1; \
		fi; \
	done
	@echo "$(GREEN)All ABIs exported to ./abis/$(NC)"


# Clean all build artifacts
.PHONY: clean
clean:
	@echo "$(GREEN)Cleaning build artifacts...$(NC)"
	@for contract in $(CONTRACTS); do \
		echo "$(YELLOW)Cleaning $$contract...$(NC)"; \
		cd $$contract && cargo clean && cd ..; \
	done
	@cd interfaces && cargo clean && cd ..
	@rm -rf abis/
	@echo "$(GREEN)Clean complete!$(NC)"


# Generate lock files for all projects
.PHONY: generate-lockfiles
generate-lockfiles:
	@echo "$(GREEN)Generating Cargo.lock files...$(NC)"
	@for contract in $(CONTRACTS); do \
		echo "$(YELLOW)Generating lockfile for $$contract...$(NC)"; \
		cd $$contract && cargo generate-lockfile && cd ..; \
	done
	@cd interfaces && cargo generate-lockfile && cd ..
	@echo "$(GREEN)Lock files generated!$(NC)"


# Environment check
.PHONY: check-env
check-env:
	@if [ ! -f .env.local ]; then \
		echo "$(RED)Error: .env.local file not found!$(NC)"; \
		echo "Please copy .env.example to .env.local and fill in your values"; \
		exit 1; \
	fi


# Deploy all contracts to devnet (localhost)
.PHONY: deploy-devnet
deploy-devnet: check-env
	@echo "$(GREEN)Deploying all contracts to devnet...$(NC)"
	@if [ -z "$(DEVNET_RPC_URL)" ] || [ -z "$(DEVNET_PRIVATE_KEY)" ]; then \
		echo "$(RED)Error: DEVNET_RPC_URL and DEVNET_PRIVATE_KEY must be set in .env.local$(NC)"; \
		exit 1; \
	fi
	@for contract in $(CONTRACTS); do \
		echo "$(YELLOW)Deploying $$contract...$(NC)"; \
		cd $$contract && cargo stylus deploy \
			--endpoint="$(DEVNET_RPC_URL)" \
			--private-key="$(DEVNET_PRIVATE_KEY)"; \
		cd ..; \
	done
	@echo "$(GREEN)All contracts deployed to devnet!$(NC)"


# Deploy all contracts to testnet (Arbitrum Sepolia)
.PHONY: deploy-testnet
deploy-testnet: check-env
	@echo "$(GREEN)Deploying all contracts to testnet...$(NC)"
	@if [ -z "$(TESTNET_RPC_URL)" ] || [ -z "$(TESTNET_PRIVATE_KEY)" ]; then \
		echo "$(RED)Error: TESTNET_RPC_URL and TESTNET_PRIVATE_KEY must be set in .env.local$(NC)"; \
		exit 1; \
	fi
	@for contract in $(CONTRACTS); do \
		echo "$(YELLOW)Deploying $$contract...$(NC)"; \
		cd $$contract && cargo stylus deploy \
			--endpoint="$(TESTNET_RPC_URL)" \
			--private-key="$(TESTNET_PRIVATE_KEY)"; \
		cd ..; \
	done
	@echo "$(GREEN)All contracts deployed to testnet!$(NC)"

# Run tests
.PHONY: test
test:
	@echo "$(GREEN)Running tests...$(NC)"
	@for contract in $(CONTRACTS); do \
		echo "$(YELLOW)Testing $$contract...$(NC)"; \
		cd $$contract && cargo test && cd ..; \
	done
	@cd interfaces && cargo test && cd ..
	@echo "$(GREEN)All tests passed!$(NC)"


# Format code
.PHONY: fmt
fmt:
	@echo "$(GREEN)Formatting code...$(NC)"
	@for contract in $(CONTRACTS); do \
		cd $$contract && cargo fmt && cd ..; \
	done
	@cd interfaces && cargo fmt && cd ..
	@cd frontend && bunx prettier --write . && cd ..
	@echo "$(GREEN)Code formatted!$(NC)"

# Run clippy
.PHONY: clippy
clippy:
	@echo "$(GREEN)Running clippy...$(NC)"
	@for contract in $(CONTRACTS); do \
		echo "$(YELLOW)Clippy $$contract...$(NC)"; \
		cd $$contract && cargo clippy --target $(WASM_TARGET) --lib -- -W clippy::all && cd ..; \
	done
	@cd interfaces && cargo clippy --lib -- -W clippy::all && cd ..
	@echo "$(GREEN)Clippy checks passed!$(NC)"


####### Individual contract targets #######
.PHONY: $(addprefix check-,$(CONTRACTS))
$(addprefix check-,$(CONTRACTS)):
	@contract=$(patsubst check-%,%,$@); \
	echo "$(YELLOW)Checking $$contract...$(NC)"; \
	cd $$contract && cargo stylus check

# Individual contract deployment targets
.PHONY: $(addprefix deploy-devnet-,$(CONTRACTS))
$(addprefix deploy-devnet-,$(CONTRACTS)): check-env
	@contract=$(patsubst deploy-devnet-%,%,$@); \
	if [ -z "$(DEVNET_RPC_URL)" ] || [ -z "$(DEVNET_PRIVATE_KEY)" ]; then \
		echo "$(RED)Error: DEVNET_RPC_URL and DEVNET_PRIVATE_KEY must be set in .env.local$(NC)"; \
		exit 1; \
	fi; \
	echo "$(YELLOW)Deploying $$contract...$(NC)"; \
	cd $$contract && cargo stylus deploy \
		--endpoint="$(DEVNET_RPC_URL)" \
		--private-key="$(DEVNET_PRIVATE_KEY)"

.PHONY: $(addprefix deploy-testnet-,$(CONTRACTS))
$(addprefix deploy-testnet-,$(CONTRACTS)): check-env
	@contract=$(patsubst deploy-testnet-%,%,$@); \
	if [ -z "$(TESTNET_RPC_URL)" ] || [ -z "$(TESTNET_PRIVATE_KEY)" ]; then \
		echo "$(RED)Error: TESTNET_RPC_URL and TESTNET_PRIVATE_KEY must be set in .env.local$(NC)"; \
		exit 1; \
	fi; \
	echo "$(YELLOW)Deploying $$contract...$(NC)"; \
	cd $$contract && cargo stylus deploy \
		--endpoint="$(TESTNET_RPC_URL)" \
		--private-key="$(TESTNET_PRIVATE_KEY)"

.PHONY: $(addprefix build-,$(CONTRACTS))
$(addprefix build-,$(CONTRACTS)):
	@contract=$(patsubst build-%,%,$@); \
	echo "$(YELLOW)Building $$contract...$(NC)"; \
	cd $$contract && cargo build --target $(WASM_TARGET) --release

.PHONY: $(addprefix export-abi-,$(CONTRACTS))
$(addprefix export-abi-,$(CONTRACTS)):
	@contract=$(patsubst export-abi-%,%,$@); \
	echo "$(YELLOW)Exporting ABI for $$contract...$(NC)"; \
	mkdir -p abis; \
	cd $$contract && cargo stylus export-abi > ../abis/$$contract.json


# Development workflow targets
.PHONY: dev
dev: fmt clippy check
	@echo "$(GREEN)Development checks complete!$(NC)"

.PHONY: ci
ci: fmt clippy test check build
	@echo "$(GREEN)CI pipeline complete!$(NC)"


################# Frontend commands #################
.PHONY: frontend-dev
frontend-dev:
	@echo "$(GREEN)Starting frontend development server...$(NC)"
	@cd frontend && bun dev

.PHONY: frontend-build
frontend-build:
	@echo "$(GREEN)Building frontend...$(NC)"
	@cd frontend && bun run build

.PHONY: frontend-install
frontend-install:
	@echo "$(GREEN)Installing frontend dependencies...$(NC)"
	@cd frontend && bun install


########### Full project commands ###########
.PHONY: install
install: setup frontend-install
	@echo "$(GREEN)Full project setup complete!$(NC)"

.PHONY: all
all: clean fmt clippy test check build export-abi
	@echo "$(GREEN)Full build pipeline complete!$(NC)"
