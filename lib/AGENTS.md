# AGENTS.md - stignore-lib

Shared core library crate for the stignore workspace, providing foundational data structures, configuration parsing, validation engines, and agent/manager communication protocols.

## Key Components

### Core Modules
- `types.rs` - Core data structures including `ItemGroup`, `Item`, `Category`, `SortOrder`, `AgentInfo`, and protocol request/response structs
- `config.rs` - TOML configuration structures (`AgentConfig`, `ManagerConfig`), environment variable expansion (`${VAR}` and `$VAR`), and config file path resolution
- `lib.rs` - `.stignore` syntax validation (`validate_stignore_content`), safety rules, and content hashing (`compute_content_hash`) for optimistic locking

### Key Data Structures & Logic
- **`ItemGroup`**: Hierarchical filesystem tree model implementing `Add` for additive merging across nodes, conflict detection, active sync tracking, and copy counting
- **`SortOrder`**: Sorting engine supporting `name_asc`, `name_desc`, `size_desc`, and `size_asc`
- **`.stignore` Validation Engine**: Detects dangerous patterns (e.g. root deletions) and invalid include syntax
- **Content Hashing**: Deterministic hashing for optimistic concurrency control during `.stignore` edits

## Development Commands

```bash
# Build library
cargo build --package stignore-lib

# Run library tests
cargo test --package stignore-lib

# Format and lint
cargo fmt --check
cargo clippy --all-targets --all-features
```
