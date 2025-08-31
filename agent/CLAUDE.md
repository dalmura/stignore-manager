# CLAUDE.md - stignore-agent

## Project Overview
This is a Rust-based HTTP API agent that provides a JSON API for managing file system items and `.stignore` files. The agent serves as an interface between a manager system and the local filesystem, allowing categorized browsing and ignore file management.

**Part of the stignore workspace** - uses shared types and configuration from `stignore-lib`.

## Architecture
- **Language**: Rust (edition 2021)
- **Web Framework**: Axum 0.8.4
- **Runtime**: Tokio (async)
- **Config Format**: TOML
- **Logging**: tracing + tracing-subscriber
- **Shared Library**: Uses `stignore-lib` for types and config

## Key Components

### Core Modules
- `main.rs` - Application entry point and route configuration
- `filesystem.rs` - File system operations and item management
- `tasks.rs` - HTTP endpoint handlers

### Shared Dependencies
- Configuration loading via `stignore-lib::load_agent_config`
- Data structures from `stignore-lib::*` (ItemGroup, request/response types)
- Error handling from `stignore-lib::ConfigError`

### Configuration
The agent is configured via TOML files with the following structure:
- Agent settings (port, name, base_path, api_key)
- Categories (id, name, relative_path)

Example config files: `config.toml`, `config-agent1.toml`, etc.

#### Authentication
All API endpoints (except the help page at `/`) require authentication via the `X-API-Key` header. The API key must match the `api_key` value configured in the agent's TOML configuration file. Use a UUID for the API key value.

## API Endpoints

### Base Routes
- `GET /` - Help page with documentation link
- `GET /api/v1/categories` - List all configured categories with both id and name
- `GET /api/v1/categories/{id}` - Get specific category info

### Item Management
- `POST /api/v1/items` - Get item info via JSON payload with hierarchical path

### Ignore File Management
- `POST /api/v1/ignore` - Add item to .stignore file
- `POST /api/v1/ignore-status` - Check if item is ignored
- `POST /api/v1/ignore-status-bulk` - Check ignore status for multiple items
- `POST /api/v1/delete` - Delete item from filesystem

### API Response Format
All endpoints that return filesystem items use the `ItemGroup` structure from `stignore-lib` containing:
- `id` - Identifier (category ID for categories, filename for files/folders)
- `name` - Display name (category name for categories, filename for files/folders)
- `size_kb` - Size in kilobytes
- `items` - Child items array
- `leaf` - Boolean indicating if this is a leaf node
- `copy_count` - Number of agent copies (used by manager)

## Development Commands

### Build and Run (Workspace)
```bash
# Build from workspace root
cargo build --bin stignore-agent

# Run with config from workspace root
cargo run --bin stignore-agent stignore-agent/config.toml

# Or run binary directly
./target/debug/stignore-agent stignore-agent/config.toml
```

### Testing (Workspace)
```bash
# Run all workspace tests
cargo test

# Run only agent tests
cargo test --bin stignore-agent

# Run with backtrace for failed tests
RUST_BACKTRACE=1 cargo test
```

### Linting and Formatting (Workspace)
```bash
# Format entire workspace
cargo fmt

# Check lints for workspace
cargo clippy --all-targets --all-features
```

## Project Structure (Workspace)
```
stignore-agent/
├── src/
│   ├── main.rs           # Application entry and routing
│   ├── filesystem.rs     # File system operations
│   └── tasks.rs          # HTTP handlers
├── scripts/
│   ├── create_agents.sh  # Agent creation script
│   ├── create_fake_data.sh # Test data generation
│   └── run_agents.sh     # Multi-agent runner
├── config*.toml          # Configuration files
└── Cargo.toml           # Agent crate manifest
```

## Dependencies
- **stignore-lib** - Shared types and config (workspace library)
- **axum** - Web framework
- **tokio** - Async runtime
- **serde** - Serialization (from workspace)
- **tracing** - Logging

### Dev Dependencies
- **axum-test** - HTTP testing
- **tempfile** - Temporary file handling for tests

## Testing Strategy
The project includes comprehensive integration tests covering:
- All API endpoints (POST)
- Category management
- Item traversal using filesystem names
- Ignore file operations
- Delete operations
- Bulk operations
- Error handling scenarios

Tests use temporary directories and mock file structures to ensure isolation.

After any changes to the code please run the tests along with formatting and linting.

## Usage Notes
- Agent expects a config file path as the first command line argument
- Listens on localhost at configured port (default 3000)
- Manages `.stignore` files within category directories
- Uses filesystem names for item identification (no hashing)
- Supports hierarchical item traversal via `Vec<String>` paths
- All filesystem paths use hierarchical arrays: `["category", "folder", "subfolder"]`

## Path Format
All API endpoints that work with filesystem paths use a consistent hierarchical format:
- **Input**: `Vec<String>` representing folder hierarchy
- **Example**: `["movies", "Movie Name (2023)"]` for `/movies/Movie Name (2023)`
- **Benefits**: No URL encoding issues, supports deep hierarchies, human-readable

## Security Notes

- Currently uses HTTP (not HTTPS) for agent communication
- Runs on localhost by default
- API key authentication required for all endpoints (except help page)
- Use UUID format for API keys (e.g., `550e8400-e29b-41d4-a716-446655440000`)
- API keys are sent via `X-API-Key` header
