#!/bin/sh
set -eu

cd "$(dirname "$0")/.."   # cwd is /app from WORKDIR, but don't rely on it
./tests/setup.sh
exec cargo test "$@"
