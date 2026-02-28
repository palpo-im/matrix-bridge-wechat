#!/bin/sh
set -eu

if [ ! -f /data/config.yaml ]; then
  cp /opt/matrix-wechat/example-config.yaml /data/config.yaml
  echo "No config file found at /data/config.yaml."
  echo "A default config has been copied."
  echo "Update /data/config.yaml and restart the container."
  exit 0
fi

exec /usr/bin/matrix-wechat --config /data/config.yaml
