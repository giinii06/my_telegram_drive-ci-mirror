package main

import _ "embed"

// webUIHTML embeds the web workspace directly into the CLI executable.
//
//go:embed webui/index.html
var webUIHTML string
