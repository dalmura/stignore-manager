# stignore manager

A web dashboard for managing `.stignore` rules, tracking file redundancy, and coordinating storage across Syncthing nodes.

## Overview

Managing `.stignore` files across multiple Syncthing devices (servers, NAS, seedboxes) typically requires manual SSH sessions and text editing. `stignore-manager` connects to lightweight agents across your network to provide:

- **Redundancy tracking**: Check copy counts against a target threshold across all nodes.
- **Centralized ignore management**: Add, remove, or edit `.stignore` rules with one click.
- **Conflict detection**: Spot sync conflicts and active transfers across your cluster.
- **Media automation**: Coordinate file deletions with Radarr and Sonarr to prevent unwanted re-downloads.

## Features

- **Multi-node file browser**: View and manage synchronized files across all connected agents.
- **Copy count monitoring**: Visual warnings for files that do not meet minimum redundancy targets.
- **Ignore rule management**: Toggle sync exclusions per node or in bulk across multiple nodes.
- **Built-in rule editor**: Directly view and edit `.stignore` files with pattern syntax help.
- **Status filtering**: Quick filters for sync conflicts, active transfers, and copy counts.
- **Radarr & Sonarr integration**: Auto-cleanup, season unmonitoring, and import list exclusion support.
- **Reverse proxy auth & RBAC**: Header authentication with Admin (read/write) and Reader (read-only) roles.
- **Keyboard navigation**: Vim-style keys (`h`/`j`/`k`/`l`), search (`/`), and shortcut help (`?`).

## Quick Start

Pre-built multi-arch images are available via GitHub Container Registry.

### docker-compose.yml

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

## Configuration

### Agent (`agent-config.toml`)

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

### Manager (`manager-config.toml`)

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

# Optional: Reverse proxy auth (Authentik, Authelia, Traefik, Nginx)
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

## Media Integrations

When deleting items via the manager:
- **Auto-cleanup**: Removes the title from Radarr/Sonarr when the last copy is deleted.
- **Season support**: Deleting a season folder deletes episode files and unmonitors the season in Sonarr.
- **Import exclusions**: Adds deleted items to exclusion lists to prevent RSS/Trakt lists from re-adding them.

Settings can also be set via environment variables (e.g. `RADARR_URL`, `RADARR_API_KEY`, `SONARR_URL`, `SONARR_API_KEY`).

## Security & RBAC

- **Agent auth**: Communication requires matching `X-API-Key` headers.
- **Proxy header auth**:
  - `Admin`: Full read and write access.
  - `Reader`: Read-only access to browse files and view status.

## Keyboard Shortcuts

| Shortcut | Action |
| :--- | :--- |
| `↓` or `j` | Move selection down |
| `↑` or `k` | Move selection up |
| `→` or `l` | Drill down into folder |
| `←` or `h` | Return to parent folder |
| `Enter` | Select item / view details |
| `/` | Focus search |
| `Esc` | Clear search / close modal |
| `?` | Show help modal |

## Development

For architecture details, local build commands, and workspace overview, see [DEVELOPMENT.md](DEVELOPMENT.md).

## License

[GNU Affero General Public License v3.0](LICENSE)

