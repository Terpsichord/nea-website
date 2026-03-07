# use alpine rust
FROM rust:1.76-alpine

# create non-root user and workspace
RUN adduser -D -u 1000 runner && \
    mkdir -p /home/workspace && \
    chown runner:runner /home/workspace

# set working directory and home
WORKDIR /home/workspace
ENV HOME=/home/runner

# switch to non-root user
USER runner

CMD ["sh"]