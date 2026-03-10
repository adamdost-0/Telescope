FROM mcr.microsoft.com/playwright:v1.49.0-jammy

# Base image includes browsers at /ms-playwright and required OS deps.
ENV PLAYWRIGHT_BROWSERS_PATH=/ms-playwright \
    PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD=1

# Playwright base image already contains a non-root user (pwuser).
USER pwuser
WORKDIR /home/pwuser

ENV CARGO_HOME=/home/pwuser/.cargo \
    RUSTUP_HOME=/home/pwuser/.rustup \
    PATH=/home/pwuser/.cargo/bin:$PATH

RUN curl https://sh.rustup.rs -sSf | sh -s -- -y --default-toolchain stable

WORKDIR /repo
