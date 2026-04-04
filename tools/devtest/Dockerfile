FROM mcr.microsoft.com/playwright:v1.58.2-jammy

ARG NODE_MAJOR=22
ARG PNPM_VERSION=9.15.4

# Base image includes browsers at /ms-playwright and the runtime libraries
# Playwright needs for Chromium-based E2E runs.
ENV DEBIAN_FRONTEND=noninteractive \
    PLAYWRIGHT_BROWSERS_PATH=/ms-playwright \
    PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD=1

USER root
RUN apt-get update \
  && apt-get install -y --no-install-recommends \
    build-essential \
    ca-certificates \
    curl \
    git \
    gnupg \
    libayatana-appindicator3-dev \
    libglib2.0-dev \
    libgtk-3-dev \
    librsvg2-dev \
    libwebkit2gtk-4.1-dev \
    patchelf \
    pkg-config \
    python3 \
    sudo \
  && if [ "$(node -p 'process.versions.node.split(".")[0]')" -lt "${NODE_MAJOR}" ]; then \
    curl -fsSL "https://deb.nodesource.com/setup_${NODE_MAJOR}.x" | bash - \
    && apt-get install -y --no-install-recommends nodejs; \
  fi \
  && npm install --global corepack \
  && corepack enable \
  && echo "pwuser ALL=(ALL) NOPASSWD:ALL" >/etc/sudoers.d/99-pwuser \
  && chmod 0440 /etc/sudoers.d/99-pwuser \
  && rm -rf /var/lib/apt/lists/*

USER pwuser
WORKDIR /home/pwuser

ENV CARGO_HOME=/home/pwuser/.cargo \
    RUSTUP_HOME=/home/pwuser/.rustup \
    PATH=/home/pwuser/.cargo/bin:$PATH

RUN curl https://sh.rustup.rs -sSf | sh -s -- -y --profile minimal --default-toolchain stable \
  && rustup component add rustfmt clippy rust-analyzer rust-src \
  && corepack prepare "pnpm@${PNPM_VERSION}" --activate \
  && rustc --version \
  && cargo --version \
  && node --version \
  && npm --version \
  && pnpm --version

WORKDIR /repo
