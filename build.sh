#!/bin/sh
set -eu

cargo build --release "$@"
