# stignore System Overview

A distributed Rust-based system for managing `.stignore` files across multiple locations through a web interface and HTTP API agents.

## Architecture

This is a **Rust workspace** containing three complementary crates:

### lib/
**Purpose**: Shared library containing common types, configuration structures, and utilities used by both agent and manager

**Key Components**:
- `ItemGroup` data structure for hierarchical filesystem representation
- Configuration loading functions (`load_agent_config`, `load_manager_config`)
- Shared API request/response types for agent communication
- Error handling (`ConfigError`) and serialization support
- Type definitions for agent/manager communication protocols

### agent/
**Purpose**: HTTP API server that provides filesystem access and `.stignore` file management for a specific location

**Key Features**:
- JSON API for browsing filesystem hierarchically
- Category-based organization of file locations
- `.stignore` file creation and management
- Filesystem name-based item identification
- Configurable via TOML files

**Main Endpoints**:
- `GET /api/v1/categories` - List configured categories
- `POST /api/v1/items` - Get item information by path
- `POST /api/v1/ignore` - Add items to `.stignore` files

### manager/
**Purpose**: Web-based aggregation service that manages multiple agents and provides a unified interface

**Key Features**:
- Web UI for viewing data from multiple agents
- HTTP client for communicating with agent APIs
- Data aggregation and consolidation across agents
- HTMX-powered dynamic web interface
- Tera templating system

**Architecture**: Axum web server that proxies requests to configured agents and presents consolidated results

## Development Workflow

### Workspace Commands
```bash
# Build everything
cargo build

# Build specific binaries
cargo build --bin stignore-agent
cargo build --bin stignore-manager

# Run tests for entire workspace
cargo test

# Linting and formatting (workspace-wide)
cargo fmt
cargo clippy --all-targets --all-features
```

### Running the System
```bash
# Start Agents
cargo run --bin stignore-agent agent/config.toml

# Start Manager  
cargo run --bin stignore-manager manager/config.toml

# Access UI: Visit manager web interface to browse filesystem data from all agents
```

### Binary Locations
After building, binaries are located at:
- `target/debug/stignore-agent`
- `target/debug/stignore-manager`

## Configuration

### Agent Configuration
Located in `agent/config*.toml` - configured via TOML files specifying port, name, api_key, and categories with filesystem paths:
```toml
[agent]
port = 3000
name = "Agent Name"
base_path = "/path/to/files"
api_key = "550e8400-e29b-41d4-a716-446655440000"

[[categories]]
id = "movies"
name = "Movies"
relative_path = "movies/"
```

### Manager Configuration
Located in `manager/config.toml` - configured with manager port and list of agent hostnames/ports with matching API keys:
```toml
[manager]
port = 8000
minimum_copies = 2
agent_timeout_seconds = 5

[[agents]]
name = "Agent 1"
hostname = "localhost:3001"
api_key = "550e8400-e29b-41d4-a716-446655440000"
```

## Security
- API key authentication secures all communication between manager and agents
- Uses `X-API-Key` header with UUID-format keys (e.g., `550e8400-e29b-41d4-a716-446655440000`)
- Each agent must have a matching API key in both agent and manager configurations

## Use Cases
- Managing `.stignore` files across multiple project locations
- Centralized view of filesystem structures from different sources  
- Bulk ignore file operations across distributed repositories

## Shared Library Benefits
- **Code Reuse**: Common types, config parsing, and utilities shared between agent and manager
- **Type Safety**: Consistent API contracts enforced at compile time
- **Maintenance**: Single source of truth for data structures and protocols
- **Testing**: Unified test suite with shared test utilities
- **Versioning**: Coordinated releases of all components
