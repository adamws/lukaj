FROM ubuntu:mantic

LABEL org.opencontainers.image.source=https://github.com/adamws/lukaj
LABEL org.opencontainers.image.description="Container image for lukaj CI tests"
LABEL org.opencontainers.image.licenses=LGPL-3.0-or-later

ARG DEBIAN_FRONTEND=noninteractive

RUN apt-get update \
  && apt-get install -y \
     automake autoconf build-essential curl cmake git libtool ninja-build tar unzip zip \
     libcairo2-dev libgdk-pixbuf-2.0-dev libglib2.0-dev libpango1.0-dev \
     libsdl2-dev libsdl2-ttf-dev libxml2-dev \
     xvfb x11-xserver-utils \
  && rm -rf /var/lib/apt/lists/*

# archlinux's version of xvfb-run script is better because it includes '-d' flag
# which works better than '-a' when starting many Xfvb instances in parallel
# (which is the case in cargo tests by default)
RUN curl -o /usr/bin/xvfb-run \
  https://gitlab.archlinux.org/archlinux/packaging/packages/xorg-server/-/raw/main/xvfb-run \
  && chmod +x /usr/bin/xvfb-run

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | bash -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"
ENV CARGO_HOME="/root/.cargo"

RUN cargo install grcov cargo-vcpkg \
  && rustup component add llvm-tools-preview
