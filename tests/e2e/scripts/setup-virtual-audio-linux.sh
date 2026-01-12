#!/bin/bash
# Setup virtual audio devices on Linux using PipeWire
#
# This script creates virtual audio sink and source devices for E2E testing.
# Requires PipeWire and WirePlumber to be installed and running.
#
# Usage: ./setup-virtual-audio-linux.sh [create|destroy]

set -euo pipefail

SINK_NAME="jamjam-test-sink"
SOURCE_NAME="jamjam-test-source"

create_devices() {
    echo "Creating virtual audio devices..."

    # Check if PipeWire is running
    if ! pgrep -x "pipewire" > /dev/null; then
        echo "Error: PipeWire is not running"
        echo "Start PipeWire with: systemctl --user start pipewire pipewire-pulse"
        exit 1
    fi

    # Create virtual sink (for audio output capture)
    pw-cli create-node adapter '{
        factory.name = support.null-audio-sink
        node.name = "'"$SINK_NAME"'"
        node.description = "jamjam Test Sink"
        media.class = Audio/Sink
        audio.position = [ FL FR ]
        audio.rate = 48000
    }' 2>/dev/null || echo "Sink may already exist"

    # Create virtual source (for audio input injection)
    pw-cli create-node adapter '{
        factory.name = support.null-audio-sink
        node.name = "'"$SOURCE_NAME"'"
        node.description = "jamjam Test Source"
        media.class = Audio/Source
        audio.position = [ FL FR ]
        audio.rate = 48000
    }' 2>/dev/null || echo "Source may already exist"

    # Wait for nodes to be created
    sleep 1

    echo "Virtual audio devices created:"
    echo "  Sink: $SINK_NAME"
    echo "  Source: $SOURCE_NAME"

    # List created devices
    echo ""
    echo "Available devices:"
    pw-cli list-objects Node | grep -E "(jamjam|null-audio)" || true
}

destroy_devices() {
    echo "Destroying virtual audio devices..."

    # Find and destroy the sink
    SINK_ID=$(pw-cli list-objects Node | grep -B5 "$SINK_NAME" | grep "id:" | awk '{print $2}' | tr -d ',')
    if [ -n "$SINK_ID" ]; then
        pw-cli destroy "$SINK_ID" 2>/dev/null || true
        echo "Destroyed sink (id: $SINK_ID)"
    fi

    # Find and destroy the source
    SOURCE_ID=$(pw-cli list-objects Node | grep -B5 "$SOURCE_NAME" | grep "id:" | awk '{print $2}' | tr -d ',')
    if [ -n "$SOURCE_ID" ]; then
        pw-cli destroy "$SOURCE_ID" 2>/dev/null || true
        echo "Destroyed source (id: $SOURCE_ID)"
    fi

    echo "Virtual audio devices destroyed"
}

link_loopback() {
    echo "Creating loopback link (sink monitor -> source input)..."

    # Link sink's monitor output to source's input for loopback testing
    pw-link "${SINK_NAME}:monitor_FL" "${SOURCE_NAME}:input_FL" 2>/dev/null || echo "FL link may already exist"
    pw-link "${SINK_NAME}:monitor_FR" "${SOURCE_NAME}:input_FR" 2>/dev/null || echo "FR link may already exist"

    echo "Loopback link created"
}

status() {
    echo "Virtual audio device status:"
    echo ""
    echo "PipeWire nodes:"
    pw-cli list-objects Node | grep -E "(jamjam|null-audio)" || echo "No jamjam devices found"
    echo ""
    echo "PipeWire links:"
    pw-link -l 2>/dev/null | grep -E "(jamjam|test)" || echo "No jamjam links found"
}

usage() {
    echo "Usage: $0 [create|destroy|loopback|status]"
    echo ""
    echo "Commands:"
    echo "  create   - Create virtual audio devices"
    echo "  destroy  - Destroy virtual audio devices"
    echo "  loopback - Create loopback link between sink and source"
    echo "  status   - Show current virtual audio device status"
}

case "${1:-create}" in
    create)
        create_devices
        ;;
    destroy)
        destroy_devices
        ;;
    loopback)
        link_loopback
        ;;
    status)
        status
        ;;
    *)
        usage
        exit 1
        ;;
esac
