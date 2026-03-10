# use alpine for bash scripting
FROM alpine:3.19

# install bash and shfmt
RUN apk add --no-cache bash shfmt && \
    adduser -D -u 1000 runner && \
    mkdir -p /home/workspace && \
    chown runner:runner /home/workspace

# set working directory and home
WORKDIR /home/workspace
ENV HOME=/home/runner

# switch to non-root user
USER runner

CMD ["bash"]