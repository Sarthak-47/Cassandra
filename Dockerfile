# syntax=docker/dockerfile:1
#
# Production Cassandra CLI/proxy image. Built and pushed by
# .github/workflows/docker.yml via docker-bake.hcl, one variant per
# bake target below. All stages use the same `python:${PY_VERSION}-slim`
# base for both build and runtime (rather than manylinux, which the e2e
# Dockerfiles use) specifically so build-time and run-time glibc always
# match — no cross-distro ABI mismatch to reason about.
#
# Target matrix (must match docker-bake.hcl and the docker.yml build
# matrix exactly):
#   runtime                  -- python-slim + git/curl, root
#   runtime-nonroot          -- same, unprivileged user
#   runtime-code             -- + Node.js (coding-agent wrap support)
#   runtime-code-nonroot     -- code + unprivileged user
#   runtime-slim             -- python-slim + ca-certificates only, root
#   runtime-slim-nonroot     -- slim + unprivileged user
#   runtime-code-slim        -- slim + Node.js
#   runtime-code-slim-nonroot -- code-slim + unprivileged user

ARG PY_VERSION=3.12

# ─── Stage: builder ──────────────────────────────────────────────────
# Compile the wheel. build-essential is required: cassandra-py's build.rs
# compiles a small glibc-compat C shim (see crates/cassandra-py/Cargo.toml),
# and libsqlite3-sys's `bundled` feature builds SQLite from source.
FROM python:${PY_VERSION}-slim AS builder

ENV CARGO_HOME=/usr/local/cargo \
    RUSTUP_HOME=/usr/local/rustup \
    PATH=/usr/local/cargo/bin:${PATH} \
    DEBIAN_FRONTEND=noninteractive \
    PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1

RUN apt-get update && apt-get install -y --no-install-recommends \
      build-essential \
      ca-certificates \
      curl && \
    rm -rf /var/lib/apt/lists/* && \
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
      | sh -s -- -y --no-modify-path --profile minimal -c rustfmt -c clippy --default-toolchain 1.95.0

WORKDIR /build
COPY pyproject.toml uv.lock README.md ./
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY crates/ crates/
COPY cassandra/ cassandra/

RUN pip install --no-cache-dir 'maturin>=1.5,<2.0' && \
    maturin build --release --out /dist

# ─── Stage: runtime-base ─────────────────────────────────────────────
# Shared wheel install. No apt packages beyond the base image here --
# each of the slim/full variants below adds its own layer on top.
FROM python:${PY_VERSION}-slim AS runtime-base

ENV PIP_DISABLE_PIP_VERSION_CHECK=1 \
    PIP_NO_CACHE_DIR=1 \
    PYTHONUNBUFFERED=1 \
    PYTHONDONTWRITEBYTECODE=1

COPY --from=builder /dist/*.whl /tmp/wheels/
RUN pip install --no-cache-dir "$(ls /tmp/wheels/cassandra_ai-*.whl)[proxy]" && \
    python -c "from cassandra._core import DiffCompressor, SmartCrusher; print('cassandra._core OK')" && \
    rm -rf /tmp/wheels

WORKDIR /workspace
EXPOSE 8787
# ENTRYPOINT bakes in the `proxy` subcommand -- cassandra/install/runtime.py's
# Docker-native orchestrator appends ONLY proxy flags (--host, --port, ...)
# after the image name, not "proxy" itself, expecting the entrypoint to
# already resolve to `cassandra proxy` (see the comment at that call site,
# issue #833: an appended "proxy" would double up as `cassandra proxy
# cassandra proxy ...` and Click aborts on the extra arguments). CMD is only
# the default when no override args are given (e.g. `docker run <image>`
# with nothing else).
ENTRYPOINT ["cassandra", "proxy"]
CMD ["--host", "0.0.0.0", "--port", "8787"]

# ─── Stage: runtime-slim-base ────────────────────────────────────────
# Minimal apt layer: just what TLS/cert validation needs.
FROM runtime-base AS runtime-slim-base
RUN apt-get update && apt-get install -y --no-install-recommends \
      ca-certificates && \
    rm -rf /var/lib/apt/lists/*

# ─── Stage: runtime-full-base ────────────────────────────────────────
# Fuller utility layer: adds git + curl, useful for `wrap` flows that
# shell out to install/update coding-agent CLIs.
FROM runtime-base AS runtime-full-base
RUN apt-get update && apt-get install -y --no-install-recommends \
      ca-certificates \
      curl \
      git && \
    rm -rf /var/lib/apt/lists/*

# ─── Stage: Node.js layer (for the "code" variants) ──────────────────
FROM runtime-full-base AS runtime-code-base
RUN apt-get update && apt-get install -y --no-install-recommends \
      curl \
      gnupg && \
    curl -fsSL https://deb.nodesource.com/setup_22.x | bash - && \
    apt-get install -y --no-install-recommends nodejs && \
    rm -rf /var/lib/apt/lists/*

FROM runtime-slim-base AS runtime-code-slim-base
RUN apt-get update && apt-get install -y --no-install-recommends \
      curl \
      gnupg && \
    curl -fsSL https://deb.nodesource.com/setup_22.x | bash - && \
    apt-get install -y --no-install-recommends nodejs && \
    rm -rf /var/lib/apt/lists/*

# ─── nonroot helper (repeated per final stage: USER must come after
# all root-level RUN steps for that stage) ────────────────────────────

FROM runtime-full-base AS runtime

FROM runtime-full-base AS runtime-nonroot
RUN groupadd -r cassandra && useradd -r -g cassandra -d /workspace cassandra && \
    chown -R cassandra:cassandra /workspace
USER cassandra

FROM runtime-code-base AS runtime-code

FROM runtime-code-base AS runtime-code-nonroot
RUN groupadd -r cassandra && useradd -r -g cassandra -d /workspace cassandra && \
    chown -R cassandra:cassandra /workspace
USER cassandra

FROM runtime-slim-base AS runtime-slim

FROM runtime-slim-base AS runtime-slim-nonroot
RUN groupadd -r cassandra && useradd -r -g cassandra -d /workspace cassandra && \
    chown -R cassandra:cassandra /workspace
USER cassandra

FROM runtime-code-slim-base AS runtime-code-slim

FROM runtime-code-slim-base AS runtime-code-slim-nonroot
RUN groupadd -r cassandra && useradd -r -g cassandra -d /workspace cassandra && \
    chown -R cassandra:cassandra /workspace
USER cassandra
