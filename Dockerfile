FROM golang:1.25-bookworm AS builder

WORKDIR /src
COPY go.mod go.sum ./
RUN go mod download
COPY . .
RUN CGO_ENABLED=0 GOGC=25 go build -p 1 -trimpath -ldflags="-s -w" -o /out/tdrive ./cmd/tdrive

FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates tini \
    && rm -rf /var/lib/apt/lists/*

ENV HOME=/data \
    TDRIVE_CONTAINER=1 \
    TDRIVE_NO_OS_MOUNT=1
RUN mkdir -p /data /app
COPY --from=builder /out/tdrive /usr/local/bin/tdrive
COPY docker/entrypoint.sh /usr/local/bin/tdrive-entrypoint
RUN chmod +x /usr/local/bin/tdrive-entrypoint

EXPOSE 8080
VOLUME ["/data"]

ENTRYPOINT ["/usr/bin/tini", "--", "/usr/local/bin/tdrive-entrypoint"]
