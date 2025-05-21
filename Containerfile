FROM alpine:latest AS deps
RUN apk update && apk add rust i3wm gpick xterm cargo xvfb bash

FROM alpine:latest
COPY --from=deps /var/cache/apk /var/cache/apk
COPY . ./app
WORKDIR /app
RUN apk add --no-cache rust i3wm gpick xterm cargo xvfb bash
CMD /app/tests/setup.sh && cargo test
