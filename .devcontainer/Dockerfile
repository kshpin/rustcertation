FROM mcr.microsoft.com/devcontainers/rust:1-bullseye

RUN export DEBIAN_FRONTEND=noninteractive

RUN dpkg --add-architecture arm64 && \
    apt-get update && \
    apt-get install --assume-yes \
    curl \
    build-essential \
    cmake \
    g++-aarch64-linux-gnu \
    git \
    pkg-config \
    libdbus-1-dev:arm64 \
    libudev-dev:arm64 \
    libxkbcommon-dev:arm64 \
    libfontconfig1-dev:arm64

RUN apt-get install -y git

# custom for this container
RUN apt-get install -y \
    cmake \
    libasound2-dev \
    libfreetype6-dev \
    pkg-config \
    libfontconfig1-dev
