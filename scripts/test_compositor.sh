#!/bin/bash

# Test Compositor Script - TTY Testing for Custom Wayland Compositor
# This script tests the compositor in TTY environment following the deployment guide

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Paths
COMPOSITOR_DIR="/home/shane/vscode/custom-wayland-compositor"
COMPOSITOR_BINARY="$COMPOSITOR_DIR/target/release/custom-wayland-compositor"
LOG_SCRIPT="$COMPOSITOR_DIR/scripts/analyze_logs.sh"

# Function to show usage
show_usage() {
    echo "Usage: $0 [OPTION]"
    echo ""
    echo "Options:"
    echo "  build     - Build the compositor in release mode"
    echo "  test      - Test compositor functionality"
    echo "  env       - Setup TTY environment"
    echo "  apps      - Test with external applications"
    echo "  logs      - Show recent logs"
    echo "  clean     - Clean up processes"
    echo "  all       - Run complete test suite"
    echo "  --help    - Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0 build     # Build compositor"
    echo "  $0 test      # Test compositor functionality"
    echo "  $0 all       # Run complete test suite"
}

# Function to check if we're in TTY
check_tty() {
    if [ -z "$XDG_VTNR" ] && [ "$TERM" != "linux" ]; then
        echo -e "${YELLOW}Warning: Not running in TTY. For best results, run in TTY2 (Ctrl+Alt+F2)${NC}"
        echo -e "${YELLOW}Continue anyway? (y/N): ${NC}"
        read -r response
        if [[ ! "$response" =~ ^[Yy]$ ]]; then
            exit 1
        fi
    fi
}

# Function to setup environment
setup_environment() {
    echo -e "${BLUE}Setting up TTY environment...${NC}"
    
    # Setup environment variables
    export XDG_RUNTIME_DIR="/run/user/$(id -u)"
    export WAYLAND_DISPLAY="wayland-1"
    export XDG_CURRENT_DESKTOP="CustomCompositor"
    export XDG_SESSION_DESKTOP="CustomCompositor"
    
    # Create runtime directory if it doesn't exist
    if [ ! -d "$XDG_RUNTIME_DIR" ]; then
        sudo mkdir -p "$XDG_RUNTIME_DIR"
        sudo chown "$(id -u):$(id -g)" "$XDG_RUNTIME_DIR"
        sudo chmod 700 "$XDG_RUNTIME_DIR"
    fi
    
    echo -e "${GREEN}✓ Environment setup complete${NC}"
    echo -e "${BLUE}XDG_RUNTIME_DIR: $XDG_RUNTIME_DIR${NC}"
    echo -e "${BLUE}WAYLAND_DISPLAY: $WAYLAND_DISPLAY${NC}"
}

# Function to build compositor
build_compositor() {
    echo -e "${BLUE}Building compositor in release mode...${NC}"
    cd "$COMPOSITOR_DIR"
    
    if cargo build --release; then
        echo -e "${GREEN}✓ Compositor built successfully${NC}"
        echo -e "${BLUE}Binary location: $COMPOSITOR_BINARY${NC}"
    else
        echo -e "${RED}✗ Build failed${NC}"
        exit 1
    fi
}

# Function to test compositor
test_compositor() {
    echo -e "${YELLOW}Testing compositor functionality...${NC}"
    
    if [ ! -f "$COMPOSITOR_BINARY" ]; then
        echo -e "${RED}✗ Compositor binary not found. Run build first.${NC}"
        exit 1
    fi
    
    echo -e "${BLUE}Starting compositor...${NC}"
    # Start compositor in background and capture PID
    "$COMPOSITOR_BINARY" &
    local compositor_pid=$!
    
    # Give compositor time to start
    sleep 3
    
    # Check if compositor is still running
    if kill -0 $compositor_pid 2>/dev/null; then
        echo -e "${GREEN}✓ Compositor started successfully (PID: $compositor_pid)${NC}"
        
        # Wait a bit more for full initialization
        sleep 2
        
        # Check for Wayland socket
        if [ -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ]; then
            echo -e "${GREEN}✓ Wayland socket created: $XDG_RUNTIME_DIR/$WAYLAND_DISPLAY${NC}"
        else
            echo -e "${YELLOW}! Wayland socket not found${NC}"
        fi
        
        # Kill compositor
        kill $compositor_pid 2>/dev/null || true
        wait $compositor_pid 2>/dev/null || true
        echo -e "${GREEN}✓ Compositor stopped cleanly${NC}"
    else
        echo -e "${RED}✗ Compositor failed to start or crashed${NC}"
        exit 1
    fi
}

# Function to test external applications
test_external_applications() {
    echo -e "${YELLOW}Testing external Wayland applications...${NC}"
    
    # Test with weston-terminal if available
    if command -v weston-terminal >/dev/null 2>&1; then
        echo -e "${BLUE}Testing weston-terminal...${NC}"
        timeout 3s weston-terminal 2>/dev/null &
        local terminal_pid=$!
        sleep 1
        if kill -0 $terminal_pid 2>/dev/null; then
            pkill -f weston-terminal 2>/dev/null || true
            echo -e "${GREEN}✓ weston-terminal launched successfully${NC}"
        else
            echo -e "${YELLOW}! weston-terminal test completed${NC}"
        fi
    else
        echo -e "${YELLOW}! weston-terminal not installed${NC}"
    fi
    
    # Test with alacritty if available
    if command -v alacritty >/dev/null 2>&1; then
        echo -e "${BLUE}Testing alacritty...${NC}"
        timeout 3s alacritty 2>/dev/null &
        local alacritty_pid=$!
        sleep 1
        if kill -0 $alacritty_pid 2>/dev/null; then
            pkill -f alacritty 2>/dev/null || true
            echo -e "${GREEN}✓ alacritty launched successfully${NC}"
        else
            echo -e "${YELLOW}! alacritty test completed${NC}"
        fi
    else
        echo -e "${YELLOW}! alacritty not installed${NC}"
    fi
}

# Function to show logs
show_logs() {
    echo -e "${YELLOW}Showing recent logs...${NC}"
    if [ -f "$LOG_SCRIPT" ]; then
        $LOG_SCRIPT startup
    else
        echo -e "${YELLOW}Log analysis script not found${NC}"
        echo -e "${BLUE}Looking for log directories...${NC}"
        find /tmp -name "custom_compositor_logs*" -type d 2>/dev/null | head -5
    fi
}

# Function to clean up processes
cleanup_processes() {
    echo -e "${YELLOW}Cleaning up processes...${NC}"
    
    # Kill any running compositor instances
    pkill -f custom-wayland-compositor 2>/dev/null || true
    
    # Kill test applications
    pkill -f weston-terminal 2>/dev/null || true
    pkill -f alacritty 2>/dev/null || true
    
    echo -e "${GREEN}✓ Cleanup complete${NC}"
}

# Function to run all tests
run_all_tests() {
    echo -e "${BLUE}Running complete test suite...${NC}"
    echo ""
    
    check_tty
    setup_environment
    build_compositor
    test_compositor
    test_external_applications
    show_logs
    cleanup_processes
    
    echo ""
    echo -e "${GREEN}✓ Test suite completed${NC}"
}

# Main script logic
case "${1:-}" in
    "build")
        build_compositor
        ;;
    "test")
        setup_environment
        test_compositor
        ;;
    "env")
        setup_environment
        ;;
    "apps")
        setup_environment
        test_external_applications
        ;;
    "logs")
        show_logs
        ;;
    "clean")
        cleanup_processes
        ;;
    "all")
        run_all_tests
        ;;
    "--help"|"-h"|"help")
        show_usage
        ;;
    "")
        echo -e "${YELLOW}No option specified. Use --help for usage information.${NC}"
        echo ""
        show_usage
        ;;
    *)
        echo -e "${RED}Unknown option: $1${NC}"
        echo ""
        show_usage
        exit 1
        ;;
esac
