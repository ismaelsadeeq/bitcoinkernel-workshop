FROM debian:bookworm-slim

# Install build tools and compilers
RUN apt-get update && apt-get install -y \
    # Build tools
    make \
    cmake \
    ninja-build \
    ccache \
    \
    # Compilers and toolchain
    gcc \
    g++ \
    clang \
    libclang-dev \
    llvm-dev \
    pkg-config \
    \
    # Build utilities
    autoconf \
    automake \
    libtool \
    bison \
    patch \
    \
    # Version control
    git \
    \
    # Python development
    python3 \
    python3-dev \
    python3-pip \
    \
    # Compression and utilities
    xz-utils \
    rsync \
    procps \
    util-linux \
    \
    # Linux-specific
    linux-libc-dev \
    \
    # Bitcoin Core dependencies
    libboost-all-dev \
    libevent-dev \
    libzmq3-dev \
    libsqlite3-dev \
    libsodium-dev \
    libdb-dev \
    libdb++-dev \
    libcapnp-dev \
    capnproto \
    \
    # Code editor
    vim  \
    nano \
    # Additional utilities
    curl \
    e2fsprogs \
    dash \
    wget \
    && rm -rf /var/lib/apt/lists/*

# Install Rust toolchain using rustup
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable

# Add Rust to PATH
ENV PATH="/root/.cargo/bin:${PATH}"

# Install additional Rust components
RUN rustup component add rustfmt clippy rust-analyzer

# Set environment variables
ENV CC=gcc \
    CXX=g++ \
    CMAKE_GENERATOR=Ninja \
    RUST_BACKTRACE=1 \
    LIBCLANG_PATH=/usr/lib/llvm-14/lib

# Verify installations
RUN echo "=== Build Environment ===" && \
    echo "GCC: $(gcc --version | head -n1)" && \
    echo "G++: $(g++ --version | head -n1)" && \
    echo "Clang: $(clang --version | head -n1)" && \
    echo "CMake: $(cmake --version | head -n1)" && \
    echo "Ninja: $(ninja --version)" && \
    echo "Git: $(git --version)" && \
    echo "Rust: $(rustc --version)" && \
    echo "Cargo: $(cargo --version)" && \
    echo "Capnp: $(capnp --version)" && \
    echo "========================="

# Set working directory
WORKDIR /workspace

# Default command
CMD ["/bin/bash"]
