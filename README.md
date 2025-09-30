# Ethos - Decentralized Reputation System

[![Built with Stylus](https://img.shields.io/badge/Built%20with-Stylus-red.svg)](https://arbitrum.io/stylus)
[![Rust](https://img.shields.io/badge/rust-1.70+-blue.svg)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

**Ethos** is a decentralized reputation system built on Arbitrum Stylus that enables users to build verifiable digital identities through Soulbound Tokens (SBTs). Unlike traditional systems where reputation can be bought, Ethos creates an identity that you *earn* through actions and achievements.

## 🏗️ Project Structure

```bash
ethos/
├── README.md                       # This file
├── Makefile                        # Development automation
├── .env.example                    # Environment template
├── .env.local                      # Your actual config (ignored)
├── .gitignore                      # Git ignore rules
├── issuer-registry/                # Individual Stylus project
│   ├── Cargo.toml
│   ├── rust-toolchain.toml
│   └── src/lib.rs
├── sbt/                           # Individual Stylus project
│   ├── Cargo.toml
│   ├── rust-toolchain.toml
│   └── src/lib.rs
└── sbt-factory/                   # Individual Stylus project
    ├── Cargo.toml
    ├── rust-toolchain.toml
    └── src/lib.rs
```

## 🎯 Vision & Core Features

### Vision

Create a platform where DAOs, educational institutions, event organizers, and protocols can issue verifiable, non-transferable tokens that collectively form a rich, multi-faceted on-chain reputation system.

### Core Features

1. **🏛️ Issuer Registry**: Whitelist of legitimate organizations that can issue SBTs
2. **🏭 SBT Factory**: Allows registered issuers to deploy new SBT collections
3. **🏅 Soulbound Tokens**: Non-transferable ERC-721 tokens representing achievements

## Deployed Contracts

- SBT [0x503783260866cAe2547c49211cf9dD9c739bA677](https://sepolia.arbiscan.io/address/0x503783260866cae2547c49211cf9dd9c739ba677)
- SBT FACTORY [0x7da6b1fc8197c8745c629d708d17583a3856de5e](https://sepolia.arbiscan.io/address/0x7da6b1fc8197c8745c629d708d17583a3856de5e)
- ISSUER REGISTRY [0xa6b51a44d3f9f4eb3010fa0f643dbe7bf95b58c3](https://sepolia.arbiscan.io/address/0xa6b51a44d3f9f4eb3010fa0f643dbe7bf95b58c3)
  
## Frontend Repo

- [Ethos frontend](https://github.com/Psalmuel01/arb-token-minter)

## 🚀 Getting Started

### Prerequisites

- **Rust** 1.89.0+ ([Install Rust](https://rustup.rs/))
- **Node.js** 18+ ([Install Node.js](https://nodejs.org/))
- **Bun** ([Install Bun](https://bun.sh/))
- **Docker** (for local devnet) ([Install Docker](https://docs.docker.com/get-docker/))
- **Git** for version control

### Quick Setup

1. **Clone and setup**

   ```bash
   git clone https://github.com/signor1/ethos.git
   cd ethos
   make setup
   ```

2. **Configure environment**

   ```bash
   # Edit .env.local with your RPC URLs and private keys
   cp .env.example .env.local
   nano .env.local
   ```

3. **Check everything works**

   ```bash
   make check
   ```

## 🛠️ Team Development Workflow

### Project Structure Commands

```bash
# Get help - shows all available commands
make help

# Full project setup (run once)
make setup                    # Installs Rust toolchain, cargo-stylus, creates .env.local

# Generate lock files (if needed)
make generate-lockfiles       # Creates Cargo.lock for all contracts
```

### Development Commands

```bash
# Check all contracts compile to WASM
make check                    # Validates all contracts for Stylus deployment

# Check individual contract
make check-sbt               # Check specific contract
make check-issuer_registry   # Check issuer registry
make check-sbt_factory       # Check factory contract

# Build contracts to WASM
make build                   # Build all contracts
make build-sbt              # Build specific contract

# Code quality
make fmt                    # Format all Rust and frontend code
make clippy                 # Run Rust linter on all contracts
make test                   # Run all contract tests

# Development workflow
make dev                    # fmt + clippy + check (quick dev cycle)
make ci                     # Full CI pipeline: fmt + clippy + test + check + build
```

### Contract ABIs

```bash
# Export Solidity ABIs for frontend integration
make export-abi             # Export all contract ABIs to ./abis/
make export-abi-sbt         # Export specific contract ABI
```

### Deployment Commands

#### Local Development (Devnet)

```bash
# Start local Arbitrum devnet first
docker run --rm -it -p 0.0.0.0:8547:8547 offchainlabs/nitro-node:v2.3.3 --init.dev-init --node.parent-chain-reader.enable=false

# Deploy all contracts to devnet
make deploy-devnet          # Deploys all contracts to localhost:8547

# Deploy individual contracts
make deploy-devnet-issuer_registry     # Deploy issuer registry only
make deploy-devnet-sbt                 # Deploy SBT contract only
make deploy-devnet-sbt_factory         # Deploy factory only
```

#### Testnet Deployment (Arbitrum Sepolia)

```bash
# Deploy all contracts to Arbitrum Sepolia
make deploy-testnet         # Requires testnet private key in .env.local

# Deploy individual contracts to testnet
make deploy-testnet-sbt                # Deploy SBT to testnet
```

### Utility Commands

```bash
# Clean build artifacts
make clean                  # Removes all target/ directories and generated files

# Complete project workflow
make all                    # clean + fmt + clippy + test + check + build + export-abi
```

## 📋 Typical Development Workflows

### 1. Setting Up a New Environment

```bash
git clone https://github.com/signor1/ethos.git
cd ethos
make setup                  # Install everything
# Edit .env.local with your keys
make check                  # Verify setup
```

### 2. Daily Development Cycle

```bash
# Make code changes...
make dev                    # Quick validation (fmt + clippy + check)
make test                   # Run tests if you added/changed tests
```

### 3. Before Committing Code

```bash
make ci                     # Full pipeline check
```

### 4. Testing Contracts Locally

```bash
# Start devnet in terminal 1
docker run --rm -it -p 0.0.0.0:8547:8547 offchainlabs/nitro-node:v2.3.3 --init.dev-init

# Deploy contracts in terminal 2
make deploy-devnet          # Deploy all contracts
# Copy contract addresses from output

```

### 5. Deploying to Testnet

```bash
# Make sure .env.local has testnet private key
make check                  # Verify contracts compile
make deploy-testnet         # Deploy to Arbitrum Sepolia
# Copy addresses for frontend integration
```

## 🔧 Environment Configuration

Your `.env.local` file should contain:

```bash
# Local Development (using provided devnet key)
DEVNET_RPC_URL=http://localhost:8547
DEVNET_PRIVATE_KEY=0xb6b15c8cb491557369f3c7d2c287b053eb229daa9c22138887752191c9520659

# Arbitrum Sepolia Testnet
TESTNET_RPC_URL=https://sepolia-rollup.arbitrum.io/rpc
TESTNET_PRIVATE_KEY=your_testnet_private_key_here
```

## 🎯 Contract-Specific Workflows

### Issuer Registry

```bash
make check-issuer_registry           # Verify compilation
make deploy-devnet-issuer_registry   # Deploy locally
# Use address to register trusted issuers
```

### SBT Contracts

```bash
make check-sbt                       # Check SBT contract
make deploy-devnet-sbt              # Deploy SBT template
# Factory will deploy instances of this
```

### SBT Factory

```bash
make check-sbt_factory              # Check factory
make deploy-devnet-sbt_factory      # Deploy factory
# Allows registered issuers to create SBT collections
```

## 🛠️ Development Workflow

### Branch Strategy

- `main`: Production-ready code
- `develop`: Integration branch for features
- `feature/*`: Individual features
- `hotfix/*`: Critical fixes

### Issue Labels

- `🏗️ contracts`: Smart contract development
- `🎨 frontend`: UI/UX development
- `📚 docs`: Documentation
- `🐛 bug`: Bug fixes
- `✨ feature`: New features
- `🔥 critical`: High priority issues

### Pull Request Process

1. **Create feature branch**

   ```bash
   git checkout -b feature/sbt
   ```

2. **Make changes and commit**

   ```bash
   git add .
   git commit -m "feat: add SBT contract"
   ```

3. **Push and create PR**

   ```bash
   git push origin feature/sbt
   ```

4. **PR Requirements**
   - [ ] All tests pass
   - [ ] Code review by 2+ team members
   - [ ] Documentation updated
   - [ ] No merge conflicts

## 🧪 Testing Strategy

### Contract Testing

```bash
# Run all contract tests
cargo test

# Test specific contract
cargo test -p issuer-registry

# Test with coverage
cargo install cargo-tarpaulin
cargo tarpaulin --all
```

## 🔐 Security Considerations

- **Access Control**: Only registered issuers can create SBTs
- **Non-transferability**: Transfer functions explicitly disabled

## 📊 Gas Optimization

Stylus provides significant gas savings over traditional Solidity:

- **70-90% reduction** in execution costs
- **Efficient state management** with Rust's memory model
- **Lower deployment costs** with WASM compilation

### Quick Start for Contributors

1. **Pick an issue** from our [Issues tab](https://github.com/signor1/ethos/issues)
2. **Comment** on the issue to claim it
3. **Fork** the repository
4. **Create** feature branch
5. **Submit** pull request

#### Built with ❤️ using Arbitrum Stylus
