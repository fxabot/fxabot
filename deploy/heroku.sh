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
webhook_secret = "$GITHUB_WEBHOOK_SECRET"
EOF
)

echo "$config" > heroku.toml
