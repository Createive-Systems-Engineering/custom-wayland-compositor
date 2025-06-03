#!/bin/bash
# Log Analysis Tool for Custom Wayland Compositor
# Provides real-time monitoring and analysis of compositor logs

set -e

LOG_DIR="/tmp/custom_compositor_logs"
LATEST_SESSION=""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

print_header() {
    echo -e "${BLUE}=================================${NC}"
    echo -e "${BLUE}  Compositor Log Analysis Tool${NC}"
    echo -e "${BLUE}=================================${NC}"
    echo
}

find_latest_session() {
    if [ ! -d "$LOG_DIR" ]; then
        echo -e "${RED}Error: Log directory $LOG_DIR not found${NC}"
        exit 1
    fi
    
    LATEST_SESSION=$(ls -1t "$LOG_DIR" | grep "compositor_session_" | head -n 1)
    if [ -z "$LATEST_SESSION" ]; then
        echo -e "${RED}Error: No compositor sessions found in $LOG_DIR${NC}"
        exit 1
    fi
    
    echo -e "${GREEN}Latest session: $LATEST_SESSION${NC}"
    echo -e "${GREEN}Session directory: $LOG_DIR/$LATEST_SESSION${NC}"
    echo
}

show_startup_phases() {
    echo -e "${YELLOW}=== STARTUP PHASES ===${NC}"
    local startup_log="$LOG_DIR/$LATEST_SESSION/startup.log"
    
    if [ -f "$startup_log" ]; then
        while IFS= read -r line; do
            if echo "$line" | grep -q "PHASE:"; then
                phase=$(echo "$line" | sed 's/.*PHASE: \([^-]*\) -.*/\1/')
                details=$(echo "$line" | sed 's/.*PHASE: [^-]* - \(.*\)/\1/')
                timestamp=$(echo "$line" | sed 's/\[\([^]]*\)\].*/\1/')
                
                case "$phase" in
                    *"INIT"*)
                        echo -e "${CYAN}[$timestamp] $phase${NC}: $details"
                        ;;
                    *"ERROR"*)
                        echo -e "${RED}[$timestamp] $phase${NC}: $details"
                        ;;
                    *"READY"*)
                        echo -e "${GREEN}[$timestamp] $phase${NC}: $details"
                        ;;
                    *)
                        echo -e "${YELLOW}[$timestamp] $phase${NC}: $details"
                        ;;
                esac
            fi
        done < "$startup_log"
    else
        echo -e "${RED}No startup log found${NC}"
    fi
    echo
}

show_errors() {
    echo -e "${RED}=== ERRORS ===${NC}"
    local error_log="$LOG_DIR/$LATEST_SESSION/errors.log"
    
    if [ -f "$error_log" ] && [ -s "$error_log" ]; then
        cat "$error_log"
    else
        echo -e "${GREEN}No errors found${NC}"
    fi
    echo
}

show_hardware_info() {
    echo -e "${PURPLE}=== HARDWARE INFO ===${NC}"
    local hardware_log="$LOG_DIR/$LATEST_SESSION/hardware.log"
    
    if [ -f "$hardware_log" ]; then
        cat "$hardware_log"
    else
        echo -e "${RED}No hardware log found${NC}"
    fi
    echo
}

show_tracing_errors() {
    echo -e "${RED}=== TRACING ERRORS ===${NC}"
    local tracing_error_log="$LOG_DIR/$LATEST_SESSION/tracing_errors.log"
    
    if [ -f "$tracing_error_log" ] && [ -s "$tracing_error_log" ]; then
        cat "$tracing_error_log"
    else
        echo -e "${GREEN}No tracing errors found${NC}"
    fi
    echo
}

tail_main_log() {
    echo -e "${BLUE}=== MAIN LOG (last 20 lines) ===${NC}"
    local main_log="$LOG_DIR/$LATEST_SESSION/main.log"
    
    if [ -f "$main_log" ]; then
        tail -n 20 "$main_log"
    else
        echo -e "${RED}No main log found${NC}"
    fi
    echo
}

watch_logs() {
    echo -e "${BLUE}=== WATCHING LOGS (Ctrl+C to stop) ===${NC}"
    local main_log="$LOG_DIR/$LATEST_SESSION/main.log"
    
    if [ -f "$main_log" ]; then
        tail -f "$main_log"
    else
        echo -e "${RED}No main log found${NC}"
    fi
}

analyze_performance() {
    echo -e "${CYAN}=== PERFORMANCE ANALYSIS ===${NC}"
    local main_log="$LOG_DIR/$LATEST_SESSION/main.log"
    local startup_log="$LOG_DIR/$LATEST_SESSION/startup.log"
    
    if [ -f "$startup_log" ]; then
        echo "Startup timing analysis:"
        grep "PHASE:" "$startup_log" | while IFS= read -r line; do
            timestamp=$(echo "$line" | sed 's/\[\([^]]*\)\].*/\1/')
            phase=$(echo "$line" | sed 's/.*PHASE: \([^-]*\) -.*/\1/')
            echo "  $phase: $timestamp"
        done
    fi
    
    if [ -f "$main_log" ]; then
        echo
        echo "Error summary:"
        grep -c "ERROR" "$main_log" 2>/dev/null || echo "0"
        echo
        echo "Warning summary:"
        grep -c "WARN" "$main_log" 2>/dev/null || echo "0"
    fi
    echo
}

show_help() {
    echo "Usage: $0 [command]"
    echo
    echo "Commands:"
    echo "  startup     Show startup phases"
    echo "  errors      Show error log"
    echo "  hardware    Show hardware information"
    echo "  main        Show main log (last 20 lines)"
    echo "  watch       Watch logs in real-time"
    echo "  perf        Analyze performance"
    echo "  all         Show all information (default)"
    echo "  help        Show this help"
    echo
}

# Main execution
print_header
find_latest_session

case "${1:-all}" in
    "startup")
        show_startup_phases
        ;;
    "errors")
        show_errors
        show_tracing_errors
        ;;
    "hardware")
        show_hardware_info
        ;;
    "main")
        tail_main_log
        ;;
    "watch")
        watch_logs
        ;;
    "perf")
        analyze_performance
        ;;
    "all")
        show_startup_phases
        show_errors
        show_tracing_errors
        show_hardware_info
        tail_main_log
        analyze_performance
        ;;
    "help")
        show_help
        ;;
    *)
        echo -e "${RED}Unknown command: $1${NC}"
        show_help
        exit 1
        ;;
esac
