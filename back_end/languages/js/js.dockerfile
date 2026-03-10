# use alpine node
FROM node:20-alpine

# install prettier globally
RUN npm install -g prettier && \
    adduser -D -u 1000 runner && \
    mkdir -p /home/workspace && \
    chown runner:runner /home/workspace

# set working directory and home
WORKDIR /home/workspace
ENV HOME=/home/runner
ENV NODE_ENV=production

# switch to non-root user
USER runner

CMD ["sh"]