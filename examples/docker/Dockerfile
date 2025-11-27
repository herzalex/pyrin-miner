# =============================================================================
# Pyrin-Miner Docker Image
# =============================================================================
# Dieses Dockerfile erstellt ein optimiertes Image für GPU-Mining
# 
# Build:
#   docker build -t pyrin-miner:latest .
#
# Run (mit NVIDIA GPU):
#   docker run --gpus all pyrin-miner:latest -a pyrin:qz...
# =============================================================================

# Stage 1: Build
FROM rust:1.73-bullseye AS builder

# Installiere Build-Abhängigkeiten
RUN apt-get update && apt-get install -y \
    protobuf-compiler \
    libprotobuf-dev \
    ocl-icd-opencl-dev \
    clinfo \
    && rm -rf /var/lib/apt/lists/*

# CUDA Toolkit für Build (optional, für CUDA-Support)
# Für vollständigen CUDA-Support verwenden Sie das nvidia/cuda Base-Image
ARG CUDA_VERSION=11.8
ENV CUDA_HOME=/usr/local/cuda

WORKDIR /app

# Kopiere nur die Cargo-Dateien zuerst für besseres Caching
COPY Cargo.toml Cargo.lock ./
COPY plugins/cuda/Cargo.toml plugins/cuda/
COPY plugins/opencl/Cargo.toml plugins/opencl/

# Erstelle Dummy-Dateien für Abhängigkeits-Caching
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    mkdir -p src/client && \
    echo "" > src/lib.rs && \
    mkdir -p plugins/cuda/src && \
    echo "fn main() {}" > plugins/cuda/src/lib.rs && \
    mkdir -p plugins/opencl/src && \
    echo "fn main() {}" > plugins/opencl/src/lib.rs

# Baue nur die Abhängigkeiten
RUN cargo build --release || true
RUN rm -rf src plugins

# Kopiere den echten Quellcode
COPY . .

# Baue die Anwendung
RUN cargo build --release -p pyrin-miner

# Stage 2: Runtime
FROM debian:bullseye-slim

# Installiere Runtime-Abhängigkeiten
RUN apt-get update && apt-get install -y \
    libssl1.1 \
    ca-certificates \
    ocl-icd-libopencl1 \
    && rm -rf /var/lib/apt/lists/*

# Erstelle einen nicht-root Benutzer
RUN useradd -m -s /bin/bash miner

# Kopiere das Binary (als root, bevor USER gewechselt wird)
COPY --from=builder /app/target/release/pyrin-miner /usr/local/bin/

# Kopiere GPU-Plugins (optional - wird nur kopiert wenn vorhanden)
COPY --from=builder /app/target/release/libpyrin*.so /usr/local/lib/ 2>/dev/null ; exit 0

USER miner
WORKDIR /home/miner

# Setze Library Path
ENV LD_LIBRARY_PATH=/usr/local/lib:$LD_LIBRARY_PATH

# Erstelle Verzeichnisse für Konfiguration und Logs
RUN mkdir -p /home/miner/config /home/miner/logs

# Volume für Konfiguration und Logs
VOLUME ["/home/miner/config", "/home/miner/logs"]

# Gesundheitscheck
HEALTHCHECK --interval=30s --timeout=10s --start-period=60s --retries=3 \
    CMD pgrep pyrin-miner || exit 1

# Entrypoint
ENTRYPOINT ["pyrin-miner"]

# Standardargumente (werden durch docker run-Argumente überschrieben)
CMD ["--help"]
