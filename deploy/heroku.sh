#!/bin/sh

set -e

config=$(cat <<EOF
[server]
host = "0.0.0.0"
port = $PORT

[github]
username = "fxabot"
authorized = ["seanmonstar"]
token = "$GITHUB_ACCESS_TOKEN"
EOF
)

echo "$config" > heroku.toml
