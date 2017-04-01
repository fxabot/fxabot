#!/bin/sh

set -e

config=$(cat <<EOF
[server]
host = "127.0.0.1"
port = $PORT

[github]
username = "fxabot"
authorized = ["seanmonstar"]
token = "$GITHUB_ACCESS_TOKEN"
EOF
)

echo "$config" > heroku.toml
