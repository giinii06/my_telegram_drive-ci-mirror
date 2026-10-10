#!/bin/bash
# Regression test for the tdrive web gallery (prev/next across previewable
# siblings). Runs the embedded web UI in jsdom.
# Usage: ./scripts/test-web-ui.sh   (needs node; jsdom is installed into a
# throwaway dir under /tmp on first run)
set -euo pipefail
cd "$(dirname "$0")/.."
if [ ! -d /tmp/tdrive-web-ui-test/node_modules/jsdom ]; then
  mkdir -p /tmp/tdrive-web-ui-test
  (cd /tmp/tdrive-web-ui-test && npm init -y > /dev/null 2>&1 && npm i jsdom --no-audit --no-fund 2>&1 | tail -1)
fi
NODE_PATH=/tmp/tdrive-web-ui-test/node_modules WEB_UI_HTML=$PWD/cmd/tdrive/webui/index.html node scripts/web-ui-gallery.test.js
NODE_PATH=/tmp/tdrive-web-ui-test/node_modules WEB_UI_HTML=$PWD/cmd/tdrive/webui/index.html node scripts/web-ui-folder.test.js
