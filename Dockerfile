# syntax=docker/dockerfile:1

# ------------------------------------------------------
# 1) Builder stage
# ------------------------------------------------------
FROM golang:1.22-alpine3.20 AS builder

# Enable CGO, specify a musl-based toolchain, and build tags for CosmWasm
ENV CGO_ENABLED=1
ENV CC=gcc
ENV LINK_STATICALLY=true
ENV BUILD_TAGS=muslc

# Optionally adjust this if your code references a different version of wasmvm
ENV WASMVM_VERSION="v2.1.2"

WORKDIR /app
COPY . .

# Install the Alpine packages needed to build a static musl binary
RUN apk add --no-cache \
    build-base \
    binutils-gold \
    musl-dev \
    git \
    wget

# Download the musl-based "libwasmvm_muslc.x86_64.a" from the WasmVM repo
RUN set -eux; \
    LIBWASM="libwasmvm_muslc.x86_64.a"; \
    wget "https://github.com/CosmWasm/wasmvm/releases/download/${WASMVM_VERSION}/${LIBWASM}" \
    -O "/lib/${LIBWASM}"; \
    # rename/move it so the build can find it easily
    cp "/lib/${LIBWASM}" "/lib/libwasmvm_muslc.a"

# Build with full static linkage
# -linkmode external -extldflags "-static" forces a completely static binary
RUN go build \
    -tags "$BUILD_TAGS" \
    -ldflags '-linkmode external -extldflags "-static"' \
    -o xiond \
    ./cmd/xiond


# ------------------------------------------------------
# 2) Final stage: Minimal runtime image
# ------------------------------------------------------
FROM alpine:3.20

# Copy the statically-linked xiond from the builder
COPY --from=builder /app/xiond /usr/bin/xiond

# Optional: install small debugging tools (bash, jq)
RUN apk add --no-cache bash jq \
    && mkdir -p /var/cosmos-chain \
    && chown -R 1000:1000 /var/cosmos-chain

WORKDIR /var/cosmos-chain

# Common Cosmos ports: REST(1317), P2P(26656), RPC(26657), gRPC(9090)
EXPOSE 1317 26656 26657 9090

# Default command just runs xiond
CMD ["/usr/bin/xiond"]

##CONT move on with polytone workshop

################OLD NOTES#############


##Try an ubuntu based buil

##Maybe I need to take a step back and review what local-ic needs and why it
##seems to be incompatible with XION Dockerfile