#!/bin/zsh
# DEPRECATED: Use ../play.sh instead
# This script is kept for compatibility but redirects to the main launcher

echo "🔄 Redirecting to main launcher..."
exec "$(dirname "$0")/../play.sh" "$@"
