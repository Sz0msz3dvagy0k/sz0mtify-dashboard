#!/bin/sh
set -eu

api_base_escaped=$(printf '%s' "${FRONTEND_API_BASE_URL:-}" | sed 's/\\/\\\\/g; s/"/\\"/g')

cat > /usr/share/nginx/html/config.js <<EOF
window.__ARCHIVE_CONFIG__ = {
	apiBaseUrl: "$api_base_escaped"
};
EOF
