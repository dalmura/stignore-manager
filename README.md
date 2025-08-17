# stignore-manager
stignore manager v2 written in Rust

## Running
```
cargo run config.toml
```

## Containers
By default the container attempts to load `/app/config.toml`, if you don't want this just specify a different config file as the first parameter.

Build:
```
docker build -t stignore-manager:latest .
```

Run:
```
# Default config location
docker run -it --rm --name stignore-manager -v "$(pwd)/config-manager.toml:/app/config.toml" -p 8000:8000 -e RUST_LOG=info stignore-manager:latest

# Custom config location
docker run -it --rm --name stignore-manager -v "$(pwd)/config-manager.toml:/app/config-custom.toml" -p 8000:8000 -e RUST_LOG=info stignore-manager:latest /app/config-custom.toml
```
