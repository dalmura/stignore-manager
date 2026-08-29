# stignore System Overview

A distributed Rust-based system for managing `.stignore` rules, tracking file redundancy, monitoring sync health, and coordinating media management across Syncthing nodes.

## Architecture

This repository is structured as a **Rust workspace** consisting of three crates:

```text
.
├── lib/                  # Shared library crate (stignore-lib)
├── agent/                # Lightweight storage node agent (stignore-agent)
├── manager/              # Centralized web dashboard & aggregator (stignore-manager)
├── scripts/              # Local development & mock data generation scripts
└── .github/workflows/    # CI/CD workflows for testing and container releases
```

---

### 1. `lib/` (`stignore-lib`)
**Purpose**: Shared core library containing common data structures, configuration parsing, validation engines, and agent/manager communication protocols.

**Key Components**:
- **`ItemGroup`**: Hierarchical filesystem tree model.
  - Implements `std::ops::Add` for additive merging across multiple agents.
  - Computes consolidated folder sizes, leaf node status, and copy counts.
  - Redundancy checking via `has_insufficient_copies(minimum_copies)`.
  - Syncthing state metadata:
    - `has_conflicts` & `conflict_count`: Detects `.sync-conflict-*` files.
    - `is_syncing`: Detects active temporary transfer files (`.syncthing.*.tmp`).
    - `stversions_size_kb`: Tracks storage consumed by `.stversions` archives.
    - `stfolder_present`: Verifies presence of Syncthing `.stfolder` directory markers.
- **`SortOrder`**: Sorting engine supporting `name_asc`, `name_desc`, `size_desc`, `size_asc`.
- **`.stignore` Validation Engine**:
  - `validate_stignore_content`: Parses `.stignore` rules and detects syntax errors or dangerous wildcard patterns (e.g. root wildcard `/` deletions, malformed includes).
  - `compute_content_hash`: Generates deterministic hashes for optimistic concurrency locking.
- **Configuration & Environment Expansion**:
  - `expand_env_vars`: Expands `${VAR}` and `$VAR` environment variable placeholders in TOML configs.
  - `resolve_config_path`: Resolves configuration file location from CLI args, `STIGNORE_CONFIG`, `CONFIG_PATH`, or standard search paths.
  - `apply_manager_env_overrides`: Applies `STIGNORE_*` environment variable overrides to manager configurations.
- **Protocol Types**: Shared request and response models for all agent HTTP endpoints.

---

### 2. `agent/` (`stignore-agent`)
**Purpose**: Lightweight HTTP API daemon deployed alongside Syncthing storage directories on each node.

**Key Features**:
- Hierarchical filesystem inspection with configurable category base paths.
- Syncthing metadata inspection (conflict detection, active transfers, `.stversions` size, `.stfolder` verification).
- `.stignore` rule insertion (`#include` and pattern rules) and removal.
- Direct `.stignore` file retrieval and replacement with optimistic locking (`expected_hash`) and automatic timestamped backups.
- File and directory deletion on disk.
- Graceful shutdown handling on `SIGINT` (Ctrl+C) and `SIGTERM`.
- API key authentication via `X-API-Key` header.

**API Endpoints**:
| Method | Route | Description |
| :--- | :--- | :--- |
| `GET` | `/` | Help and documentation link (unauthenticated) |
| `GET` | `/api/v1/categories` | List all configured categories |
| `GET` | `/api/v1/categories/{id}` | Retrieve items within a specific category |
| `POST` | `/api/v1/items` | Retrieve item information for a hierarchical folder path (`item_path: Vec<String>`) |
| `POST` | `/api/v1/ignore` | Add ignore rule to the category's `.stignore` file |
| `POST` | `/api/v1/unignore` | Remove ignore rule from the category's `.stignore` file |
| `POST` | `/api/v1/ignore-status` | Check if a specific path is ignored |
| `POST` | `/api/v1/ignore-status-bulk` | Batch check ignore status for multiple paths |
| `POST` | `/api/v1/delete` | Delete a file or directory from disk |
| `POST` | `/api/v1/stignore/get` | Get raw `.stignore` contents, content hash, and list of backups |
| `POST` | `/api/v1/stignore/set` | Update `.stignore` contents with optimistic lock verification and backup creation |
| `POST` | `/api/v1/stignore/restore` | Restore `.stignore` from an existing backup file |

---

### 3. `manager/` (`stignore-manager`)
**Purpose**: Central web aggregation service providing a unified dashboard, multi-agent coordination, and media server integrations.

**Key Features**:
- **Async Concurrency**: Parallel agent queries via `tokio::task::JoinSet` for low-latency aggregated directory browsing.
- **Dynamic HTMX UI**: Tera templating with partial HTMX components for smooth, SPA-like interactions without heavy frontend frameworks.
- **Syncthing Health Monitoring**: Real-time visibility into sync conflicts, active file transfers, and `.stversions` archive sizes across all nodes.
- **Interactive `.stignore` Editor**: Web modal for editing raw `.stignore` files with live syntax validation, optimistic locking conflict detection, and backup restoration.
- **Media Server Integrations**:
  - **Radarr**: Movie matching, auto-removal on last copy deletion, and optional import list exclusions.
  - **Sonarr**: Series & season matching, automatic season unmonitoring, and optional import list exclusions.
- **Reverse Proxy Auth & RBAC**:
  - Header-based authentication (`X-Proxy-User`, `X-Proxy-Role`).
  - Roles: `Admin` (full read/write access) and `Reader` (read-only access; mutating endpoints return `403 Forbidden`).
- **Dynamic Agent Management**: In-memory enabling/disabling of agents with live agent health status pill in the footer.
- **Vim Keyboard Navigation**: Keyboard shortcuts (`h`/`j`/`k`/`l`, `/` search, `?` help, `Esc`, `Enter`).
- **Toast Notifications**: Built-in visual feedback for async actions (success/error toasts).

**Manager Web Routes & HTMX Endpoints**:
| Method | Route | Description |
| :--- | :--- | :--- |
| `GET` | `/` | Main dashboard page |
| `GET` | `/agents` | Agents overview page (status, enabled state, categories, health) |
| `GET` | `/health`, `/healthz` | Health check endpoints |
| `GET` | `/components/itemlist.html` | Aggregated item table component |
| `GET` | `/components/dynamic-items.html` | Lazy-loaded item subtrees |
| `POST` | `/components/infopanel.html` | Item details panel with agent breakdown and action buttons |
| `POST` | `/components/agent-modal.html` | Modal displaying per-agent distribution for an item |
| `POST` | `/components/stignore-modal.html` | Modal for viewing, editing, and restoring `.stignore` files |
| `POST` | `/components/agents/toggle` | Dynamically toggle an agent enabled/disabled state |
| `GET` | `/components/agents-table.html` | Agent status table for the overview page |
| `GET` | `/components/agent-status-pill.html` | Footer status pill showing online/total agents |
| `POST` | `/components/ignore` | Add ignore rule on a specific agent |
| `POST` | `/components/unignore` | Remove ignore rule on a specific agent |
| `POST` | `/components/delete` | Delete file/folder on a specific agent |
| `POST` | `/components/delete-details` | Preview deletion impact (including Radarr/Sonarr implications) |
| `POST` | `/components/bulk-ignore` | Ignore item across all or selected agents |
| `POST` | `/components/bulk-unignore` | Unignore item across all or selected agents |
| `POST` | `/components/bulk-delete` | Bulk delete item across all agents hosting it |
| `POST` | `/components/stignore/save` | Save edited `.stignore` with optimistic locking and validation |
| `POST` | `/components/stignore/restore` | Restore `.stignore` from backup |
| `POST` | `/components/stignore/validate` | Real-time syntax and rule safety check |

---

## Configuration Reference

Both agent and manager support TOML configuration files with environment variable interpolation (`${VAR_NAME}` or `$VAR_NAME`).

### Agent Configuration (`agent/config.toml`)
```toml
[agent]
port = 3000
name = "NAS-01"
base_path = "/data"
api_key = "550e8400-e29b-41d4-a716-446655440000"

[[categories]]
id = "movies"
name = "Movies"
relative_path = "movies/"

[[categories]]
id = "tv"
name = "TV Shows"
relative_path = "tv/"
```

### Manager Configuration (`manager/config.toml`)
```toml
[manager]
port = 8000
minimum_copies = 2
agent_timeout_seconds = 5

# Optional: Reverse proxy header authentication & RBAC
[manager.auth]
enabled = true                  # Defaults to false (all requests treated as Admin)
user_header = "X-Proxy-User"    # Username header
role_header = "X-Proxy-Role"    # Role/group header
admin_role = "Admin"            # Value identifying Admin users
reader_role = "Reader"          # Value identifying Reader users

# Optional: Radarr Media Integration
[integrations.radarr]
enabled = true
url = "http://localhost:7878"
api_key = "550e8400-e29b-41d4-a716-446655440000"
category_id = "movies"
delete_files = false
add_import_exclusion = false

# Optional: Sonarr Media Integration
[integrations.sonarr]
enabled = true
url = "http://localhost:8989"
api_key = "550e8400-e29b-41d4-a716-446655440000"
category_id = "tv"
delete_files = false
add_import_list_exclusion = false

[[agents]]
name = "NAS-01"
hostname = "nas01.local:3000"
api_key = "550e8400-e29b-41d4-a716-446655440000"

[[agents]]
name = "Seedbox"
hostname = "seedbox.local:3000"
api_key = "550e8400-e29b-41d4-a716-446655440000"
```

### Environment Variable Overrides
Manager settings can be overridden using environment variables:
- `STIGNORE_PORT` or `PORT`
- `STIGNORE_MINIMUM_COPIES`
- `STIGNORE_AGENT_TIMEOUT_SECONDS`
- `STIGNORE_AUTH_ENABLED`, `STIGNORE_AUTH_USER_HEADER`, `STIGNORE_AUTH_ROLE_HEADER`, `STIGNORE_AUTH_ADMIN_ROLE`, `STIGNORE_AUTH_READER_ROLE`
- `STIGNORE_RADARR_URL`, `STIGNORE_RADARR_API_KEY`, `STIGNORE_RADARR_CATEGORY`, `STIGNORE_RADARR_ENABLED`, `STIGNORE_RADARR_ADD_IMPORT_EXCLUSION`
- `STIGNORE_SONARR_URL`, `STIGNORE_SONARR_API_KEY`, `STIGNORE_SONARR_CATEGORY`, `STIGNORE_SONARR_ENABLED`, `STIGNORE_SONARR_ADD_IMPORT_LIST_EXCLUSION`
- `STIGNORE_CONFIG` or `CONFIG_PATH`: Path to manager configuration file.

---

## Security & Authentication

1. **Agent Authentication**:
   - Every API request from the manager to an agent requires an `X-API-Key` header matching the agent's configured `api_key`.
2. **Reverse Proxy Auth & RBAC**:
   - Header authentication compatible with Authentik, Authelia, Traefik, Pomerium, and Nginx.
   - `Admin`: Full permissions (browse, ignore, unignore, delete, bulk actions, edit `.stignore`).
   - `Reader`: Read-only permissions (browse filesystem, view info panel, view modals). Any mutation attempt returns `403 Forbidden`.
   - When auth is disabled (`enabled = false`), all requests default to `Admin`.

---

## Development & Release Workflow

### Workspace Commands
```bash
# Build entire workspace
cargo build

# Build specific binaries
cargo build --bin stignore-agent
cargo build --bin stignore-manager

# Run all workspace tests (unit + integration)
cargo test

# Workspace linting and formatting
cargo fmt --check
cargo clippy --all-targets --all-features
```

### Local Test Scripts
Located in `scripts/`:
- `create_agents.sh`: Generates multi-agent test configuration files.
- `create_fake_data.sh`: Creates sample directory hierarchies with fake movie and TV files.
- `run_agents.sh`: Launches test agents in background processes.
- `run_manager.sh`: Runs the manager pointing to local test agents.
- `fresh_start.sh`: Cleans and resets the local test environment.

### Release Workflow
1. Update version in root `Cargo.toml` (`[workspace.package] version = "X.Y.Z"`).
2. Validate locally (syncs `Cargo.lock`):
   ```bash
   cargo fmt --check && cargo clippy --all-targets --all-features && cargo test
   ```
3. Commit and push:
   ```bash
   git commit -am "chore: bump version to vX.Y.Z" && git push origin main
   ```
4. Create and push git tag:
   ```bash
   git tag vX.Y.Z && git push --tags
   ```
5. GitHub Actions (`.github/workflows/cd.yaml`) will automatically build multi-arch container images, publish to GHCR, and create GitHub Releases.
