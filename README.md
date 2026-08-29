# stignore-manager

A web dashboard for managing `.stignore` rules, tracking file redundancy, and monitoring sync health across Syncthing nodes.

## Features

- **Multi-node browsing & redundancy tracking**: Aggregated directory view across nodes with copy-count tracking against target redundancy thresholds.
- **Ignore & sync management**: Per-node or bulk `.stignore` toggling, built-in editor with live syntax validation and backups, and detection for sync conflicts (`.sync-conflict-*`) and active transfers.
- **Media server integrations**: Optional Radarr and Sonarr integration for auto-cleanup, season unmonitoring, and import list exclusions on deletion.
- **Reverse proxy auth & RBAC**: Header-based authentication (`X-Proxy-User`, `X-Proxy-Role`) supporting `Admin` and `Reader` roles.
- **Vim-style navigation**: Fast keyboard navigation (`h`/`j`/`k`/`l`, `/` search, `Esc`, `?`).

## Quick Start

```yaml
# docker-compose.yml
services:
  stignore-manager:
    image: ghcr.io/dalmura/stignore-manager:latest
    container_name: stignore-manager
    restart: unless-stopped
    ports:
      - "8000:8000"
    volumes:
      - ./manager-config.toml:/app/config.toml:ro

  stignore-agent:
    image: ghcr.io/dalmura/stignore-agent:latest
    container_name: stignore-agent
    restart: unless-stopped
    ports:
      - "3000:3000"
    volumes:
      - ./agent-config.toml:/app/config.toml:ro
      - /path/to/data:/data:rw
```

## Configuration

Both manager and agent support TOML configuration files with `${VAR}` / `$VAR` environment variable expansion.

### Agent (`agent-config.toml`)

Deploy an agent on each node hosting Syncthing directories:

```toml
[agent]
port = 3000
name = "NAS-01"
base_path = "/data"
api_key = "secret-agent-key"

[[categories]]
id = "movies"
name = "Movies"
relative_path = "movies/"

[[categories]]
id = "tv"
name = "TV Shows"
relative_path = "tv/"
```

### Manager (`manager-config.toml`)

```toml
[manager]
port = 8000
minimum_copies = 2
agent_timeout_seconds = 5

[[agents]]
name = "NAS-01"
hostname = "nas01.local:3000"
api_key = "secret-agent-key"

[[agents]]
name = "Seedbox"
hostname = "seedbox.local:3000"
api_key = "secret-agent-key"

# Optional: Reverse proxy auth (e.g. Authentik, Authelia, Traefik)
[manager.auth]
enabled = false
user_header = "X-Proxy-User"
role_header = "X-Proxy-Role"
admin_role = "Admin"
reader_role = "Reader"

# Optional: Radarr integration
[integrations.radarr]
enabled = false
url = "http://radarr.local:7878"
api_key = "your-radarr-api-key"
category_id = "movies"
add_import_exclusion = true

# Optional: Sonarr integration
[integrations.sonarr]
enabled = false
url = "http://sonarr.local:8989"
api_key = "your-sonarr-api-key"
category_id = "tv"
add_import_list_exclusion = true
```

### Environment Variables

Manager settings can be overridden via environment variables:

| Variable | Description | Default |
| :--- | :--- | :--- |
| `STIGNORE_PORT` / `PORT` | Listening port | `8000` |
| `STIGNORE_MINIMUM_COPIES` | Redundancy copy threshold | `2` |
| `STIGNORE_AGENT_TIMEOUT_SECONDS` | Agent request timeout in seconds | `5` |
| `STIGNORE_CONFIG` / `CONFIG_PATH` | Path to configuration file | `config.toml` |
| `STIGNORE_AUTH_ENABLED` | Enable reverse proxy authentication | `false` |
| `STIGNORE_AUTH_USER_HEADER` | Header containing username | `X-Proxy-User` |
| `STIGNORE_AUTH_ROLE_HEADER` | Header containing user role | `X-Proxy-Role` |
| `STIGNORE_AUTH_ADMIN_ROLE` | Admin role name | `Admin` |
| `STIGNORE_AUTH_READER_ROLE` | Reader role name | `Reader` |
| `STIGNORE_RADARR_ENABLED` | Enable Radarr integration | `false` |
| `STIGNORE_RADARR_URL` | Radarr instance URL | — |
| `STIGNORE_RADARR_API_KEY` | Radarr API key | — |
| `STIGNORE_RADARR_CATEGORY` | Category ID mapped to Radarr | `movies` |
| `STIGNORE_RADARR_ADD_IMPORT_EXCLUSION` | Add deleted items to import exclusions | `false` |
| `STIGNORE_SONARR_ENABLED` | Enable Sonarr integration | `false` |
| `STIGNORE_SONARR_URL` | Sonarr instance URL | — |
| `STIGNORE_SONARR_API_KEY` | Sonarr API key | — |
| `STIGNORE_SONARR_CATEGORY` | Category ID mapped to Sonarr | `tv` |
| `STIGNORE_SONARR_ADD_IMPORT_LIST_EXCLUSION` | Add deleted items to import exclusions | `false` |

## Keyboard Shortcuts

| Shortcut | Action |
| :--- | :--- |
| `j` / `k` or `↓` / `↑` | Move selection |
| `l` / `h` or `→` / `←` | Drill down / return to parent |
| `Enter` | Select item / view details |
| `/` | Focus search filter |
| `Esc` | Clear search / close modal |
| `?` | Show shortcuts modal |

## Development

See [DEVELOPMENT.md](DEVELOPMENT.md) for build commands, architecture details, and local test scripts.

## License

[GNU Affero General Public License v3.0](LICENSE)
