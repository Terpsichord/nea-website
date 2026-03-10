# use minimal debian for c compilation
FROM debian:stable-slim

# install gcc and clang-format
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        gcc \
        clang-format \
        libc6-dev \
    && rm -rf /var/lib/apt/lists/*

# create non-root user and workspace
RUN useradd -m -u 1000 runner && \
    mkdir -p /home/workspace && \
    chown runner:runner /home/workspace

# set working directory and home
WORKDIR /home/workspace
ENV HOME=/home/runner

# switch to non-root user
USER runner

CMD ["bash"]