# Build TDrive web and CLI

```sh
mkdir -p build/bin
go build -trimpath -o build/bin/tdrive ./cmd/tdrive
build/bin/tdrive web --port 8080
```

The web workspace is embedded from `cmd/tdrive/webui/index.html`. Rebuild the CLI and restart the web service after editing it. Node is needed only for lint and browser checks, not production builds.

```sh
npm ci --prefix cmd/tdrive/webui
npm --prefix cmd/tdrive/webui run lint
bash scripts/test-web-ui.sh
go test ./cmd/tdrive ./cmd/tdrived ./backend/...
bash scripts/package-cli-release.sh v2.0.0
```

On Windows, use `scripts/package-cli-release-windows.ps1`. Releases contain CLI executables with the web UI, install helpers, and checksums.
