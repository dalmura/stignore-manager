# stignore-manager

A Rust web application that manages and aggregates data from multiple stignore agents via HTTP/JSON API endpoints.

**Part of the stignore workspace** - uses shared types and configuration from `stignore-lib`.

## Project Overview

This is a web-based manager application that:
- Communicates with multiple stignore agents over HTTP
- Aggregates and consolidates data from agents  
- Provides a web interface for viewing agent data
- Built with Axum web framework and Tera templating

## Architecture

- **Web Server**: Axum-based HTTP server with static file serving
- **Templating**: Tera template engine with HTML templates
- **Configuration**: TOML-based configuration for manager and agents
- **Agent Communication**: HTTP/JSON API calls to configured agents
- **Shared Library**: Uses `stignore-lib` for types and config

## Key Components

### Core Modules
- `main.rs` - Application entry point and server setup
- `config.rs` - Configuration loading wrapper (uses stignore-lib)
- `agents.rs` - Agent communication and data aggregation
- `pages.rs` - Web page handlers
- `components.rs` - Component routing
- `agent_client.rs` - HTTP client for agent communication

### Shared Dependencies
- Configuration loading via `stignore-lib::load_manager_config`
- Data structures from `stignore-lib::*` (ItemGroup, Agent, ManagerData, etc.)
- Agent API types for communication protocols

### Configuration
- `config.toml` - Runtime configuration
- Defines manager port and list of agents with hostnames and API keys
- Example configuration in `config.toml.example`

#### Authentication
The manager authenticates with agents using API keys sent via the `X-API-Key` header. Each agent in the configuration must have a matching `api_key` field that corresponds to the agent's configured API key. Use UUID format for API keys.

### Web Interface
- HTML templates in `html/` directory
- Static assets (CSS, JS) served from `assets/`
- HTMX for dynamic interactions
- Bootstrap toast notifications for user feedback

## API Endpoints

The manager aggregates data from agent endpoints:
- `GET /api/v1/categories` - List categories from agents
- `POST /api/v1/items` - Get item information from agents

## Data Model

**ItemGroup**: Hierarchical data structure representing items (from stignore-lib)
- Supports merging data from multiple agents
- Automatic size calculation and sorting
- Tree-like structure with leaf detection
- Copy count calculation: each agent contributes 1 copy per item
- Note: Ignore status checking for minimum copies is disabled for performance

## Development Commands

### Build and Run (Workspace)
```bash
# Build from workspace root
cargo build --bin stignore-manager

# Run with config from workspace root
cargo run --bin stignore-manager stignore-manager/config.toml

# Or run binary directly
./target/debug/stignore-manager stignore-manager/config.toml
```

### Testing (Workspace)
```bash
# Run all workspace tests
cargo test

# Run only manager tests
cargo test --lib stignore-manager
```

### Linting and Formatting (Workspace)
```bash
# Format entire workspace
cargo fmt

# Check lints for workspace
cargo clippy --all-targets --all-features
```

### Configuration
Copy and customize the example config:
```bash
cp stignore-manager/config.toml.example stignore-manager/config.toml
```

## Dependencies

- **stignore-lib**: Shared types and config (workspace library)
- **axum**: Web framework
- **axum-template**: Template integration  
- **tera**: Template engine
- **tokio**: Async runtime
- **reqwest**: HTTP client for agent communication
- **serde**: Serialization/deserialization (from workspace)
- **tracing**: Logging and instrumentation

## Testing Strategy
After any changes to the code please run the tests along with formatting and linting from the workspace root.

## UI/UX Guidelines

### User Feedback
For all user actions (ignore, delete, etc.), always use toast notifications to provide feedback:
- **Success actions**: Green toast with checkmark icon, auto-hide after 3 seconds
- **Error actions**: Red toast with X icon, auto-hide after 5 seconds  
- **Toast positioning**: Fixed to top-right corner of screen
- **Implementation**: Use the `showToast(message, type)` JavaScript function in `utils/scripts.html`

Example usage:
```javascript
// Success
showToast('Item deleted successfully', 'success');

// Error
showToast('Failed to delete item: ' + errorMessage, 'error');
```

Avoid using browser `alert()` popups or modal error messages for user action feedback.

## Agent Integration

Agents must implement compatible HTTP/JSON API endpoints:
- Categories listing endpoint
- Item information endpoint with POST requests
- Proper JSON response formats matching `AgentCategoryListingResponse` and `AgentItemInfoResponse` from `stignore-lib`

## Security Notes

- Currently uses HTTP (not HTTPS) for agent communication
- Runs on localhost by default
- API key authentication implemented for agent communication
- Use UUID format for API keys (e.g., `550e8400-e29b-41d4-a716-446655440000`)
- API keys are sent via `X-API-Key` header to agents
