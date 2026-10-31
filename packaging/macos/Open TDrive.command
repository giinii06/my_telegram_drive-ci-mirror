#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")"
if [ -x ./open-tdrive.sh ]; then
  exec bash ./open-tdrive.sh
fi
exec bash "$HOME/Applications/TDrive/bin/open-tdrive.sh"
