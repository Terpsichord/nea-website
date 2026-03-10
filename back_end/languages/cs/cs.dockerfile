# use alpine-based dotnet sdk
FROM mcr.microsoft.com/dotnet/sdk:8.0-alpine

# create non-root user
RUN adduser -D -u 1000 runner && \
    mkdir -p /home/workspace && \
    chown runner:runner /home/workspace

# set working directory and home
WORKDIR /home/workspace
ENV HOME=/home/runner
ENV DOTNET_CLI_TELEMETRY_OPTOUT=1
ENV DOTNET_SKIP_FIRST_TIME_EXPERIENCE=1

# switch to non-root user
USER runner

CMD ["sh"]