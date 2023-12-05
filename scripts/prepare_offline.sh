#!/usr/bin/env bash
set -x
set -eo pipefail

if ! [ -x "$(command -v sqlx)" ]; then
	echo >&2 "Error: sqlx is not installed."
	echo >&2 "Use:"
	echo >&2 " cargo install sqlx-cli \
	--no-default-features --features rustls,postgres"
	echo >&2 "to install it."
	exit 1
fi

cargo sqlx prepare --workspace
