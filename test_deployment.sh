#!/bin/bash
# Quick test deployment script for the Custom Wayland Compositor

set -e

echo "=== Custom Wayland Compositor Test Deployment ==="
echo "This script will help you test the compositor safely"
echo

# Check if we're already in a Wayland session
if [ -n "$WAYLAND_DISPLAY" ]; then
    echo "⚠️  You're currently in a Wayland session!"
    echo "   For safest testing, switch to a TTY first:"
    echo "   Press Ctrl+Alt+F2, login, then run this script"
    echo
    read -p "Continue anyway? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# Build the compositor
echo "Building compositor..."
cargo build --release

# Create runtime directory if needed
export XDG_RUNTIME_DIR=${XDG_RUNTIME_DIR:-/run/user/$(id -u)}
if [ ! -d "$XDG_RUNTIME_DIR" ]; then
    echo "Creating runtime directory: $XDG_RUNTIME_DIR"
    mkdir -p "$XDG_RUNTIME_DIR"
fi

# Set up environment
export WAYLAND_DISPLAY=wayland-custom
export XDG_SESSION_TYPE=wayland
export QT_QPA_PLATFORM=wayland
export GDK_BACKEND=wayland

echo
echo "=== Starting Custom Wayland Compositor ==="
echo "Environment:"
echo "  XDG_RUNTIME_DIR: $XDG_RUNTIME_DIR"
echo "  WAYLAND_DISPLAY: $WAYLAND_DISPLAY"
echo
echo "The compositor will start shortly."
echo "To test it:"
echo "  1. Open another terminal (Ctrl+Alt+F3)"
echo "  2. Export the same environment variables"
echo "  3. Run test clients: cargo run --bin shm_basic_window"
echo
echo "To stop the compositor: Ctrl+C"
echo

# Start the compositor
exec ./target/release/custom-wayland-compositor
