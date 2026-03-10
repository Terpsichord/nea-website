# use alpine eclipse temurin jdk
FROM eclipse-temurin:21-jdk-alpine

# install google-java-format
RUN apk add --no-cache curl && \
    curl -L -o /usr/local/bin/google-java-format \
      https://github.com/google/google-java-format/releases/download/v1.22.0/google-java-format-1.22.0-all-deps.jar && \
    echo '#!/bin/sh\nexec java -jar /usr/local/bin/google-java-format "$@"' \
      > /usr/local/bin/google-java-format && \
    chmod +x /usr/local/bin/google-java-format

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