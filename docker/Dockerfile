FROM ubuntu:24.04

ARG DEBIAN_FRONTEND=noninteractive

RUN apt-get update -qq && apt-get install -y -qq \
    git \
    gh \
    curl \
    build-essential \
    tmux \
    ttyd \
    zsh \
    neovim \
    libevent-dev \
    libncurses-dev \
    autoconf \
    automake \
    pkg-config \
    bison \
    libtool \
    luarocks \
    lua5.1 \
    liblua5.1-0-dev \
    ca-certificates \
    python3 && \
    rm -rf /var/lib/apt/lists/*

RUN useradd -m -s /bin/bash uwu

WORKDIR /app
COPY . /app

RUN bash -lc "set -euo pipefail; \
    curl -fsSL https://bun.sh/install | bash; \
    curl https://sh.rustup.rs -sSf | sh -s -- -y; \
    export PATH=\"/root/.bun/bin:/root/.cargo/bin:$PATH\"; \
    cd /app/tmux; \
    sh autogen.sh; \
    ./configure --prefix=/opt/uwu/tmux --disable-utf8proc; \
    make -j\"$(nproc)\"; \
    make install; \
    /root/.bun/bin/bun install --cwd /app/opencode; \
    /root/.bun/bin/bun install --cwd /app/opencode/packages/opencode; \
    /root/.bun/bin/bun install --cwd /app/oh-my-opencode; \
    /root/.cargo/bin/cargo build --manifest-path /app/daemon/Cargo.toml --release"

RUN ln -sf /root/.bun/bin/bun /usr/local/bin/bun

RUN mkdir -p /data/workspaces/workspace-1 /data/state /home/uwu/.config/uwu && \
    chown -R uwu:uwu /app /data /home/uwu/.config

COPY docker/entrypoint.sh /usr/local/bin/uwu-entrypoint
RUN chmod +x /usr/local/bin/uwu-entrypoint

USER uwu

ENV HOME=/home/uwu
ENV PATH=/home/uwu/.bun/bin:/home/uwu/.cargo/bin:/opt/uwu/tmux/bin:/usr/local/bin:/usr/bin:/bin

EXPOSE 18080
EXPOSE 7681

ENTRYPOINT ["/usr/local/bin/uwu-entrypoint"]
