FROM mcr.microsoft.com/playwright:v1.49.0-jammy

# Install Rust (stable) in container (non-root)
RUN useradd -m -u 1000 dev
USER dev
WORKDIR /home/dev

ENV CARGO_HOME=/home/dev/.cargo \
    RUSTUP_HOME=/home/dev/.rustup \
    PATH=/home/dev/.cargo/bin:$PATH

RUN curl https://sh.rustup.rs -sSf | sh -s -- -y --default-toolchain stable

# Node is already present in Playwright image; ensure npm available.
WORKDIR /repo
