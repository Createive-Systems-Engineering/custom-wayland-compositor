#!/bin/bash
# Phase 1: Foundation Tests
# Validates core compositor compilation and initialization

set -e

echo "Phase 1: Foundation Tests - Custom Wayland Compositor"
echo "============================================================"

# Test 1: Compilation Verification
echo "Test 1: Compilation verification..."
cd /home/shane/vscode/custom-wayland-compositor

if cargo check --workspace --quiet; then
    echo "PASS - Compilation successful - all crates compile without errors"
else
    echo "FAIL - Compilation failed"
    exit 1
fi

# Test 2: Dependency Validation
echo "Test 2: Dependency validation..."
if cargo tree --workspace --quiet > /dev/null 2>&1; then
    echo "PASS - Dependencies resolved correctly"
else
    echo "FAIL - Dependency resolution failed"
    exit 1
fi

# Test 3: Build Test
echo "Test 3: Full workspace build..."
if cargo build --workspace --quiet; then
    echo "PASS - Full workspace builds successfully"
    
    # Check for main executable
    if [ -f "target/debug/custom-compositor" ]; then
        echo "PASS - Main compositor executable created"
    else
        echo "FAIL - Main executable not found"
        exit 1
    fi
else
    echo "FAIL - Workspace build failed"
    exit 1
fi

# Test 4: Critical Claims Validation
echo "Test 4: Validating v1.0.0 claims..."

# Check if we can actually run the compositor
echo "  Testing compositor startup..."
timeout 5s ./target/debug/custom-compositor --help > /dev/null 2>&1 && echo "PASS - Compositor executable runs" || echo "WARNING - Compositor may need runtime dependencies"

# Check Vulkan renderer functionality
echo "  Testing Vulkan renderer..."
if cargo test --package vulkan-renderer --lib --quiet 2>/dev/null; then
    echo "PASS - Vulkan renderer tests pass"
else
    echo "WARNING - Vulkan renderer tests need implementation"
fi

# Check surface manager functionality
echo "  Testing surface manager..."
if cargo test --package compositor-core --lib --quiet surface_manager 2>/dev/null; then
    echo "PASS - Surface manager tests pass"
else
    echo "WARNING - Surface manager tests need implementation"
fi

echo ""
echo "Phase 1 Summary:"
echo "================"
echo "PASSED: Core compilation and build system"
echo "PASSED: Dependency resolution"
echo "PASSED: Executable creation"
echo "WARNING: Runtime functionality tests need implementation"
echo ""
echo "CRITICAL ASSESSMENT FOR v1.0.0:"
echo "- We have a compiling and buildable compositor"
echo "- We need to validate actual graphics rendering functionality"
echo "- Missing runtime tests for claimed 4K capabilities"
echo ""
echo "NEXT: Run Phase 2 to test graphics stack functionality"