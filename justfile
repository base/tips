### DEVELOPMENT COMMANDS ###
ci:
    # Rust
    cargo fmt --all -- --check
    cargo clippy -- -D warnings
    cargo build
    cargo test
    # UI
    cd ui && npm run lint
    cd ui && npm run build

fix:
    # Rust
    cargo fmt --all
    cargo clippy --fix --allow-dirty --allow-staged
    # UI
    cd ui && npx biome check --write --unsafe

create-migration name:
    touch crates/datastore/migrations/$(date +%s)_{{ name }}.sql

sync: deps-reset
    ### DATABASE ###
    cargo sqlx prepare -D postgresql://postgres:postgres@localhost:5432/postgres --workspace --all --no-dotenv
    cd ui && npx drizzle-kit pull --dialect=postgresql --url=postgresql://postgres:postgres@localhost:5432/postgres
    cd ui && mv ./drizzle/relations.ts ./src/db/
    cd ui && mv ./drizzle/schema.ts ./src/db/
    cd ui && rm -rf ./drizzle
    ###   ENV    ###
    just sync-env
    ###    REFORMAT   ###
    just fix

sync-env:
    cp .env.example .env
    cp .env.example ./ui/.env
    cp .env.example .env.docker
    # Change kafka ports
    sed -i '' 's/localhost:9092/host.docker.internal:9094/g' ./.env.docker
    # Change other dependencies
    sed -i '' 's/localhost/host.docker.internal/g' ./.env.docker

stop-all profiles="default":
    export COMPOSE_FILE=docker-compose.yml:docker-compose.tips.yml && export COMPOSE_PROFILES={{ profiles }} && docker compose down && docker compose rm && rm -rf data/

# Start every service running in docker, useful for demos
start-all profiles="default": (stop-all profiles)
    export COMPOSE_FILE=docker-compose.yml:docker-compose.tips.yml && export COMPOSE_PROFILES={{ profiles }} && mkdir -p data/postgres data/kafka data/minio && docker compose build && docker compose up -d 

# Stop only the specified service without stopping the other services or removing the data directories
stop-only program:
    export COMPOSE_FILE=docker-compose.yml:docker-compose.tips.yml && docker compose down {{ program }}

# Start only the specified service without stopping the other services or removing the data directories
start-only program:
    export COMPOSE_FILE=docker-compose.yml:docker-compose.tips.yml && mkdir -p data/postgres data/kafka data/minio && docker compose build && docker compose up -d {{ program }}

# Start every service in docker, except the one you're currently working on. e.g. just start-except ui ingress-rpc
start-except programs: stop-all
    #!/bin/bash
    all_services=(postgres kafka kafka-setup minio minio-setup ingress-rpc ingres-writer audit maintenance ui)
    exclude_services=({{ programs }})
    
    # Create result array with services not in exclude list
    result_services=()
    for service in "${all_services[@]}"; do
        skip=false
        for exclude in "${exclude_services[@]}"; do
            if [[ "$service" == "$exclude" ]]; then
                skip=true
                break
            fi
        done
        if [[ "$skip" == false ]]; then
            result_services+=("$service")
        fi
    done
    
    export COMPOSE_FILE=docker-compose.yml:docker-compose.tips.yml && mkdir -p data/postgres data/kafka data/minio && docker compose build && docker compose up -d ${result_services[@]}

### RUN SERVICES ###
deps-reset:
    COMPOSE_FILE=docker-compose.yml:docker-compose.tips.yml docker compose down && docker compose rm && rm -rf data/ && mkdir -p data/postgres data/kafka data/minio && docker compose up -d

deps:
    COMPOSE_FILE=docker-compose.yml:docker-compose.tips.yml docker compose down && docker compose rm && docker compose up -d

audit:
    cargo run --bin tips-audit

ingress-rpc:
    cargo run --bin tips-ingress-rpc

maintenance:
    cargo run --bin tips-maintenance

ingress-writer:
    cargo run --bin tips-ingress-writer

simulator:
    cargo run --bin tips-simulator node

simulator-playground:
    RUST_LOG=debug cargo run --bin tips-simulator node --builder.playground --datadir ~/.playground/devnet/tips-simulator --authrpc.port=8554

ui:
    cd ui && yarn dev

playground-env: sync-env
    #!/bin/bash
    set -euo pipefail
    BUILDER_PLAYGROUND_HOST_IP=$(docker run --rm alpine nslookup host.docker.internal | awk '/Address: / && $2 !~ /:/ {print $2; exit}')
    BUILDER_PLAYGROUND_PEER_ID=$(grep 'started p2p host' ~/.playground/devnet/logs/op-node.log | sed -n 's/.*peerID=\([^ ]*\).*/\1/p' | head -1)
    echo "" >> .env.docker
    echo "# Builder Playground P2P Configuration" >> .env.docker
    echo "BUILDER_PLAYGROUND_HOST_IP=${BUILDER_PLAYGROUND_HOST_IP}" >> .env.docker
    echo "BUILDER_PLAYGROUND_PEER_ID=${BUILDER_PLAYGROUND_PEER_ID}" >> .env.docker
    echo "OP_NODE_P2P_STATIC=/ip4/${BUILDER_PLAYGROUND_HOST_IP}/tcp/9003/p2p/${BUILDER_PLAYGROUND_PEER_ID}" >> .env.docker

# Start builder stack (builder-cl + builder, simulator-cl + simulator)
start-builder: playground-env (start-all "builder")

### BUILDER COMMANDS ###

# Build op-rbuilder docker image from a given remote/branch/tag
#
# This command integrates the tips-datastore crate into op-rbuilder for building.
# The complexity arises because:
# 1. op-rbuilder references tips-datastore as a sibling directory (../tips/crates/datastore)
# 2. tips-datastore uses workspace dependencies from the TIPS workspace
# 3. Docker build context only includes the op-rbuilder directory
#
# Solution: Copy tips-datastore into the build context and merge workspace dependencies
build-rbuilder remote="https://github.com/base/op-rbuilder" ref="tips-prototype":
    #!/bin/bash
    set -euo pipefail
    
    REMOTE="{{ remote }}"
    REF="{{ ref }}"
    JUSTFILE="{{ justfile() }}"
    JUSTFILE_DIR="{{ justfile_directory() }}"
    
    TEMP_DIR=$(mktemp -d)
    trap "rm -rf $TEMP_DIR" EXIT
    
    echo "Cloning $REMOTE ($REF)..."
    git clone --depth 1 --branch "$REF" "$REMOTE" $TEMP_DIR/op-rbuilder
    
    # Get the git revision from the cloned repo
    GIT_REV=$(cd $TEMP_DIR/op-rbuilder && git rev-parse --short HEAD)
    
    just --justfile "$JUSTFILE" --working-directory "$JUSTFILE_DIR" _build-rbuilder-common $TEMP_DIR "$REF" "$GIT_REV"

# Build op-rbuilder docker image from a local checkout
#
# The local checkout is copied to a temp directory so the original is not modified.
build-rbuilder-local local_path tag="local":
    #!/bin/bash
    set -euo pipefail
    
    TAG="{{ tag }}"
    JUSTFILE="{{ justfile() }}"
    JUSTFILE_DIR="{{ justfile_directory() }}"
    
    # Expand path to absolute
    LOCAL_PATH=$(cd {{ local_path }} && pwd)
    
    if [ ! -d "$LOCAL_PATH" ]; then
        echo "Error: Directory $LOCAL_PATH does not exist"
        exit 1
    fi
    
    # Get git revision and check if working tree is dirty
    cd "$LOCAL_PATH"
    GIT_REV=$(git rev-parse --short HEAD)
    if [ -n "$(git status --porcelain)" ]; then
        echo "Warning: Working tree has uncommitted changes"
        GIT_REV="${GIT_REV}-dirty"
    fi
    
    TEMP_DIR=$(mktemp -d)
    trap "rm -rf $TEMP_DIR" EXIT
    
    echo "Copying local checkout from $LOCAL_PATH (excluding generated files)..."
    mkdir -p "$TEMP_DIR/op-rbuilder"
    rsync -a \
        --exclude='target/' \
        --exclude='.git/' \
        --exclude='node_modules/' \
        --exclude='*.log' \
        --exclude='.DS_Store' \
        "$LOCAL_PATH/" "$TEMP_DIR/op-rbuilder/"
    
    just --justfile "$JUSTFILE" --working-directory "$JUSTFILE_DIR" _build-rbuilder-common $TEMP_DIR "$TAG" "$GIT_REV"

# Internal helper for building op-rbuilder docker images
_build-rbuilder-common temp_dir tag revision:
    #!/bin/bash
    set -euo pipefail
    
    TEMP_DIR="{{ temp_dir }}"
    TAG="{{ tag }}"
    REVISION="{{ revision }}"
    JUSTFILE_DIR="{{ justfile_directory() }}"
    
    echo "Setting up tips-datastore..."
    cd "$JUSTFILE_DIR"
    
    # Copy tips-datastore and its workspace Cargo.toml into the op-rbuilder directory
    # so they're included in the Docker build context
    mkdir -p "$TEMP_DIR/op-rbuilder/tips/crates"
    cp Cargo.toml "$TEMP_DIR/op-rbuilder/tips/"
    cp -r crates/datastore "$TEMP_DIR/op-rbuilder/tips/crates/"

    # Copy sqlx offline data into the datastore crate for compile-time query verification
    cp -r .sqlx "$TEMP_DIR/op-rbuilder/tips/crates/datastore/"
    
    echo "Updating workspace configuration..."
    cd "$TEMP_DIR/op-rbuilder"
    
    # Modify Dockerfile to set SQLX_OFFLINE=true in the cargo build RUN command
    # This tells sqlx to use the offline .sqlx data instead of trying to connect to a database
    sed -i '' 's/cargo build --release/SQLX_OFFLINE=true cargo build --release/g' Dockerfile
    
    # Fix the dependency path: op-rbuilder expects ../tips/crates/datastore,
    # but we copied it to tips/crates/datastore (inside the build context)
    sed -i '' 's|path = "\.\./tips/crates/datastore"|path = "tips/crates/datastore"|g' Cargo.toml
    
    # Merge workspace dependencies: tips-datastore uses .workspace = true for its dependencies,
    # which need to be defined in the workspace root. We automatically extract only the
    # dependencies that tips-datastore actually uses from the TIPS workspace and add them
    # to op-rbuilder's workspace. This keeps them in sync automatically.
    echo "" >> Cargo.toml
    echo "# TIPS workspace dependencies (auto-extracted)" >> Cargo.toml
    
    # Extract the entire [workspace.dependencies] section from TIPS for processing
    awk '/^\[workspace\.dependencies\]/,0' tips/Cargo.toml > /tmp/tips-workspace-deps.txt
    
    # Find each dependency tips-datastore uses (marked with .workspace = true)
    # and extract its full definition from TIPS, handling multiline entries
    grep "\.workspace = true" tips/crates/datastore/Cargo.toml | sed 's/\.workspace.*//' | awk '{print $1}' | while read dep; do
        if ! grep -q "^$dep = " Cargo.toml; then
            # Extract the dependency with context, stopping at the next dependency line
            # (handles multiline deps like features = [...])
            grep -A 10 "^$dep = " /tmp/tips-workspace-deps.txt | awk '/^[a-zA-Z-]/ && NR>1 {exit} {print}' >> Cargo.toml
        fi
    done
    rm -f /tmp/tips-workspace-deps.txt
    
    echo "Building docker image (revision: $REVISION)..."
    docker build -t "tips-builder:$TAG" .
    
    # Tag with git revision
    docker tag "tips-builder:$TAG" "tips-builder:$REVISION"
    
    # Tag as latest for convenience
    docker tag "tips-builder:$TAG" tips-builder:latest
    
    echo "✓ Built tips-builder:$TAG (revision: $REVISION)"
    docker images | grep tips-builder
