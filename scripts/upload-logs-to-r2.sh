#!/bin/bash
# Upload Docker container logs to Cloudflare R2
#
# This script collects logs from the last hour from all running Docker containers,
# compresses them, and uploads to Cloudflare R2.
#
# Prerequisites:
#   - rclone configured with R2 remote named "r2"
#   - Docker access (user in docker group or root)
#
# Setup rclone for R2:
#   rclone config
#   - name: r2
#   - type: s3
#   - provider: Cloudflare
#   - access_key_id: <from R2 API token>
#   - secret_access_key: <from R2 API token>
#   - endpoint: https://<account_id>.r2.cloudflarestorage.com
#
# Cron setup (hourly upload):
#   0 * * * * /opt/jamjam/scripts/upload-logs-to-r2.sh
#
# Usage:
#   ./upload-logs-to-r2.sh [--since <duration>] [--bucket <name>]

set -euo pipefail

# Configuration
R2_REMOTE="${R2_REMOTE:-r2}"
R2_BUCKET="${R2_BUCKET:-jamjam-logs}"
LOG_SINCE="${LOG_SINCE:-1h}"
TMP_DIR="/tmp/jamjam-logs"
DATE=$(date +%Y-%m-%d)
TIMESTAMP=$(date +%Y-%m-%d-%H%M%S)
HOSTNAME=$(hostname -s)

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --since)
            LOG_SINCE="$2"
            shift 2
            ;;
        --bucket)
            R2_BUCKET="$2"
            shift 2
            ;;
        --help)
            echo "Usage: $0 [--since <duration>] [--bucket <name>]"
            echo ""
            echo "Options:"
            echo "  --since   Log duration to collect (default: 1h)"
            echo "  --bucket  R2 bucket name (default: jamjam-logs)"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Check prerequisites
if ! command -v rclone &> /dev/null; then
    echo "Error: rclone not found. Install with: curl https://rclone.org/install.sh | sudo bash"
    exit 1
fi

if ! command -v docker &> /dev/null; then
    echo "Error: docker not found"
    exit 1
fi

# Check rclone config
if ! rclone listremotes | grep -q "^${R2_REMOTE}:$"; then
    echo "Error: rclone remote '${R2_REMOTE}' not configured"
    echo "Run 'rclone config' to set up R2 access"
    exit 1
fi

# Create temp directory
mkdir -p "$TMP_DIR"

# Get list of running containers
CONTAINERS=$(docker ps --format '{{.Names}}' 2>/dev/null || true)

if [ -z "$CONTAINERS" ]; then
    echo "No running containers found"
    exit 0
fi

echo "Collecting logs from containers (since: $LOG_SINCE)..."

UPLOADED=0
for CONTAINER in $CONTAINERS; do
    LOG_FILE="${TMP_DIR}/${CONTAINER}-${TIMESTAMP}.log"
    GZ_FILE="${LOG_FILE}.gz"

    # Collect logs
    if docker logs "$CONTAINER" --since "$LOG_SINCE" > "$LOG_FILE" 2>&1; then
        # Skip if empty
        if [ ! -s "$LOG_FILE" ]; then
            echo "  $CONTAINER: no logs in last $LOG_SINCE"
            rm -f "$LOG_FILE"
            continue
        fi

        # Compress
        gzip -f "$LOG_FILE"

        # Upload to R2
        R2_PATH="${R2_BUCKET}/${DATE}/${HOSTNAME}"
        if rclone copy "$GZ_FILE" "${R2_REMOTE}:${R2_PATH}/" --quiet; then
            echo "  $CONTAINER: uploaded to ${R2_PATH}/"
            UPLOADED=$((UPLOADED + 1))
        else
            echo "  $CONTAINER: upload failed"
        fi

        # Cleanup
        rm -f "$GZ_FILE"
    else
        echo "  $CONTAINER: failed to collect logs"
        rm -f "$LOG_FILE"
    fi
done

echo "Done. Uploaded $UPLOADED log file(s)."

# Cleanup temp directory if empty
rmdir "$TMP_DIR" 2>/dev/null || true
