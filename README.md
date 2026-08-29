# stignore-manager

A web dashboard for managing `.stignore` rules, tracking file redundancy, monitoring sync health, and coordinating storage across Syncthing nodes.

## Overview

Managing `.stignore` files across multiple Syncthing devices (servers, NAS, seedboxes) typically requires manual SSH sessions and text editing. `stignore-manager` connects to lightweight agents across your network to provide:

- **Redundancy tracking**: Monitor copy counts against a target threshold across all connected nodes.
- **Centralized ignore management**: Add, remove, or edit `.stignore` rules with one click.
- **Interactive `.stignore` editor**: Edit rules directly with live syntax validation, optimistic locking, and automatic backups.
- **Conflict & sync detection**: Spot sync conflicts (`.sync-conflict-*`) and active transfers (`.syncthing.*.tmp`) across your cluster.
- **Media automation**: Coordinate file deletions with Radarr and Sonarr to clean up entries, unmonitor seasons, and prevent unwanted re-downloads.
- **Reverse proxy auth & RBAC**: Header authentication with Admin (read/write) and Reader (read-only) roles.

---

## Features

- **Multi-node file browser**: Aggregated directory browsing with parallel queries across all online storage agents.
- **Copy count monitoring**: Visual warnings for files and folders that do not meet minimum redundancy targets.
- **Ignore rule management**: Toggle sync exclusions per node or in bulk across multiple nodes.
- **Built-in rule editor**: Directly view, edit, and restore `.stignore` files with safety pattern checking and automatic backups.
- **Status filtering**: Quick filters for sync conflicts, active transfers, and low redundancy copies.
- **Radarr & Sonarr integration**: Auto-cleanup, season unmonitoring, and import list exclusion support.
- **Reverse proxy auth & RBAC**: Header authentication with `Admin` (read/write) and `Reader` (read-only) roles.
- **Keyboard navigation**: Vim-style keys (`h`/`j`/`k`/`l`), search (`/`), modal close (`Esc`), and shortcut help (`?`).

---

## Quick Start

Pre-built multi-arch container images are available on GitHub Container Registry.

### `docker-compose.yml`

```yaml
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

---

## Configuration

Configuration files support environment variable interpolation using `${VAR_NAME}` or `$VAR_NAME`.

### Agent Configuration (`agent-config.toml`)

Run on each machine hosting Syncthing directories.

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

### Manager Configuration (`manager-config.toml`)

Hosts the web dashboard and connects to agents.

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

# Optional: Reverse proxy auth (Authentik, Authelia, Traefik, Pomerium, Nginx)
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
delete_files = false
add_import_exclusion = true

# Optional: Sonarr integration
[integrations.sonarr]
enabled = false
url = "http://sonarr.local:8989"
api_key = "your-sonarr-api-key"
category_id = "tv"
delete_files = false
add_import_list_exclusion = true
```

---

## Environment Variables

All manager settings can be supplied or overridden via environment variables:

| Variable | Description | Default |
| :--- | :--- | :--- |
| `STIGNORE_PORT` / `PORT` | Manager HTTP listening port | `8000` |
| `STIGNORE_MINIMUM_COPIES` | Target redundancy copy count | `2` |
| `STIGNORE_AGENT_TIMEOUT_SECONDS` | Agent request timeout in seconds | `5` |
| `STIGNORE_CONFIG` / `CONFIG_PATH` | Path to configuration file | `config.toml` |
| `STIGNORE_AUTH_ENABLED` | Enable reverse proxy header authentication | `false` |
| `STIGNORE_AUTH_USER_HEADER` | Header containing username | `X-Proxy-User` |
| `STIGNORE_AUTH_ROLE_HEADER` | Header containing user role or group | `X-Proxy-Role` |
| `STIGNORE_AUTH_ADMIN_ROLE` | Value identifying Admin role | `Admin` |
| `STIGNORE_AUTH_READER_ROLE` | Value identifying Reader role | `Reader` |
| `STIGNORE_RADARR_ENABLED` / `RADARR_ENABLED` | Enable Radarr media integration | `false` |
| `STIGNORE_RADARR_URL` / `RADARR_URL` | Radarr instance base URL | — |
| `STIGNORE_RADARR_API_KEY` / `RADARR_API_KEY` | Radarr API key | — |
| `STIGNORE_RADARR_CATEGORY` / `RADARR_CATEGORY` | Category ID mapped to Radarr | `movies` |
| `STIGNORE_RADARR_ADD_IMPORT_EXCLUSION` | Add deleted titles to import exclusion list | `false` |
| `STIGNORE_SONARR_ENABLED` / `SONARR_ENABLED` | Enable Sonarr media integration | `false` |
| `STIGNORE_SONARR_URL` / `SONARR_URL` | Sonarr instance base URL | — |
| `STIGNORE_SONARR_API_KEY` / `SONARR_API_KEY` | Sonarr API key | — |
| `STIGNORE_SONARR_CATEGORY` / `SONARR_CATEGORY` | Category ID mapped to Sonarr | `tv` |
| `STIGNORE_SONARR_ADD_IMPORT_LIST_EXCLUSION` | Add deleted series to import exclusion list | `false` |

---

## Media Integrations

When deleting items via the manager:
- **Auto-cleanup**: Removes the movie or series from Radarr/Sonarr when the last copy is deleted.
- **Season support**: Deleting a season folder removes the episode files and unmonitors the season in Sonarr.
- **Import exclusions**: Automatically adds deleted items to import exclusion lists so RSS sync and Trakt/list imports don't re-download them.

---

## Security & RBAC

- **Agent Authentication**: Communication between manager and agents is protected via `X-API-Key` headers.
- **Reverse Proxy Header Auth**:
  - `Admin`: Full read/write access (toggle ignore, delete items, edit `.stignore`).
  - `Reader`: Read-only access to browse files and view sync health status. Mutating operations are rejected with `403 Forbidden`.

---

## Keyboard Shortcuts

| Shortcut | Action |
| :--- | :--- |
| `↓` or `j` | Move selection down |
| `↑` or `k` | Move selection up |
| `→` or `l` | Drill down into folder |
| `←` or `h` | Return to parent folder |
| `Enter` | Select item / view details |
| `/` | Focus search filter |
| `Esc` | Clear search / close open modal |
| `?` | Show keyboard shortcuts modal |

---

## Development

For architecture overview, local testing guides, and workspace instructions, see [DEVELOPMENT.md](DEVELOPMENT.md).

## License

[GNU Affero General Public License v3.0](LICENSE)
