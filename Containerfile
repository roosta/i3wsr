# syntax=docker/dockerfile:1
FROM alpine:3.24.2@sha256:294b683cb724975bec92580e1e685676bd4b50bda910ddb8c51d4cabeaec77e6

RUN apk add --no-cache \
        bash cargo rust i3wm gpick xterm xvfb \
    && adduser -D -u 1000 -h /home/tester tester

WORKDIR /app
COPY --chown=tester:tester . .

USER tester
CMD ["/app/tests/run.sh"]
