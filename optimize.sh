#!/usr/bin/env bash

echo "Optimizing juno-merger..."
docker run --rm \
  -v "$(pwd)":/code \
  -w /code/contracts/juno-merger \
  --mount type=volume,source="workspace_cache",target=/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/rust-optimizer:0.16.0

# Fix permissions after Docker is done (using your current numeric user/group IDs)
sudo chown -R "$(id -u):$(id -g)" "$(pwd)/contracts/juno-merger"

echo "Optimizing xion-minter..."
docker run --rm \
  -v "$(pwd)":/code \
  -w /code/contracts/xion-minter \
  --mount type=volume,source="workspace_cache",target=/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/rust-optimizer:0.16.0

# Fix permissions for xion-minter, same idea
sudo chown -R "$(id -u):$(id -g)" "$(pwd)/contracts/xion-minter"

echo "Done!"
