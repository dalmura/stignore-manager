# stignore manager

A distributed Rust-based application for managing `.stignore` files across multiple locations through a web interface and HTTP API agents.

## Architecture

This is a Rust workspace containing three crates:

### stignore-lib/
**Purpose**: Shared library containing common types, configuration structures, and utilities

**Key Components**:
- `ItemGroup` data structure for hierarchical filesystem representation
- Configuration loading for both agent and manager
- Shared API request/response types
- Error handling and serialization

### stignore-agent/
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

### stignore-manager/
**Purpose**: Web-based aggregation service that manages multiple agents and provides a unified interface

**Key Features**:
- Web UI for viewing data from multiple agents
- HTTP client for communicating with agent APIs
- Data aggregation and consolidation across agents
- HTMX-powered dynamic web interface
- Tera templating system

## Development Workflow

### Workspace Commands
```bash
# Build everything
cargo build

# Build specific binary
cargo build --bin stignore-agent
cargo build --bin stignore-manager

# Run tests for entire workspace
cargo test

# Linting and formatting
cargo fmt
cargo clippy --all-targets --all-features
```

### Running the System
```bash
# Run agent with config
cargo run --bin stignore-agent stignore-agent/config.toml

# Run manager with config
cargo run --bin stignore-manager stignore-manager/config.toml
```

### Binary Locations
After building, binaries are located at:
- `target/debug/stignore-agent`
- `target/debug/stignore-manager`

## Configuration

### Agent Configuration
Located in `stignore-agent/config*.toml`:
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
Located in `stignore-manager/config.toml`:
```toml
[manager]
port = 8000
minimum_copies = 2
agent_timeout_seconds = 5

# Optional proxy header authentication & RBAC (Authentik, Authelia, Traefik, Nginx)
[manager.auth]
enabled = true                  # Defaults to false
user_header = "X-Proxy-User"    # Configurable, defaults to "X-Proxy-User"
role_header = "X-Proxy-Role"    # Configurable, defaults to "X-Proxy-Role"
admin_role = "Admin"            # Configurable, defaults to "Admin"
reader_role = "Reader"          # Configurable, defaults to "Reader"

[[agents]]
name = "Agent 1"
hostname = "localhost:3001"
api_key = "550e8400-e29b-41d4-a716-446655440000"
```

## Security & Authentication
- **Agent API Keys**: Uses `X-API-Key` header with matching UUID keys to secure manager-to-agent communication.
- **Proxy Header Auth & RBAC (Optional)**: Secures `stignore-manager` when placed behind a reverse proxy (e.g., Authentik):
  - Extracts username from `user_header` (e.g. `X-Proxy-User`) and assigned roles/groups from `role_header` (e.g. `X-Proxy-Role`).
  - **`Admin`**: Full access to browse filesystem data, ignore/unignore items, delete items, and toggle agents.
  - **`Reader`**: Read-only access to browse files and view agent statuses; write operations return `403 Forbidden`.
  - When disabled (`enabled = false`), all requests implicitly run with `Admin` privileges for backward compatibility.

## Use Cases
- Managing `.stignore` files across multiple project locations
- Centralized view of filesystem structures from different sources
- Bulk ignore file operations across distributed repositories

## Release Process

Container builds and GitHub releases are automated via GitHub Actions on Git tag pushes.

### 1. Update Package Versions
Update the version number in the respective crate `Cargo.toml` files:
- `manager/Cargo.toml` (`version = "x.y.z"`)
- `agent/Cargo.toml` (`version = "x.y.z"`)
- `lib/Cargo.toml` (`version = "x.y.z"`)

### 2. Verify Linting & Tests
Ensure all checks pass locally:
```bash
cargo fmt --check
cargo clippy --all-targets --all-features
cargo test
```

### 3. Commit & Push to Main
```bash
git add -A
git commit -m "chore: bump version to v1.3.2"
git push origin main
```

### 4. Create and Push a Git Tag
Push a git tag matching the version (prefixed with `v`):
```bash
git tag v1.3.2
git push origin v1.3.2
```

### 5. Automated CI/CD Pipeline
Once the tag is pushed, GitHub Actions will automatically:
1. Extract crate versions from `manager/Cargo.toml` and `agent/Cargo.toml`.
2. Build multi-arch container images (`linux/amd64`, `linux/arm64`).
3. Publish containers to GitHub Container Registry (GHCR):
   - `ghcr.io/dalmura/stignore-manager:v1.3.2` and `latest`
   - `ghcr.io/dalmura/stignore-agent:v1.1.0` and `latest`
4. Build release binaries and create a GitHub Release with automated release notes.

