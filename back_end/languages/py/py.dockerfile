# use alpine python
FROM python:3.12-alpine

# install black
RUN pip install --no-cache-dir black && \
    adduser -D -u 1000 runner && \
    mkdir -p /home/workspace && \
    chown runner:runner /home/workspace

# set working directory and home
WORKDIR /home/workspace
ENV HOME=/home/runner
ENV PYTHONDONTWRITEBYTECODE=1
ENV PYTHONUNBUFFERED=1

# switch to non-root user
USER runner

CMD ["sh"]