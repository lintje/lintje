FROM --platform=$TARGETPLATFORM alpine:3.16.0

ENV REFRESHED_AT=2022-06-18
RUN apk add --no-cache --update git

ARG TARGETARCH
COPY dist/archives/$TARGETARCH-unknown-linux-musl/lintje /usr/bin/lintje
COPY entrypoint.sh /entrypoint.sh

ENTRYPOINT ["/entrypoint.sh"]
