# What to Expect: Running Your Custom Wayland Compositor
## Realistic Expectations and Troubleshooting Guide

### What You Should Expect - The Good Scenarios

#### 1. Successful Startup (Best Case)
```bash
$ ./test_deployment.sh
=== Custom Wayland Compositor Test Deployment ===
Building compositor...
    Finished release [optimized] target(s) in 1.23s

=== Starting Custom Wayland Compositor ===
Environment:
  XDG_RUNTIME_DIR: /run/user/1000
  WAYLAND_DISPLAY: wayland-custom

[INFO] Compositor starting...
[INFO] Vulkan instance created successfully
[INFO] Surface created for display
[INFO] Wayland server socket created: wayland-custom
[INFO] Event loop starting...
[INFO] Compositor ready for clients
```

**What this means:**
- ✅ Compositor successfully initialized
- ✅ Vulkan graphics working
- ✅ Wayland socket created
- ✅ Ready for client connections

#### 2. Testing with Your Test Clients
```bash
# In another terminal (Ctrl+Alt+F3)
export WAYLAND_DISPLAY=wayland-custom
export XDG_RUNTIME_DIR=/run/user/$(id -u)

$ cargo run --bin shm_basic_window
[INFO] Connected to compositor
[INFO] Window created successfully
[INFO] Rendering frame...
```

**What you should see:**
- A simple colored window appears
- Window responds to resize attempts
- Clean shutdown when you close it

### What Could Go Wrong - Common Issues

#### 1. Vulkan/Graphics Issues
```bash
$ ./test_deployment.sh
[ERROR] Failed to create Vulkan instance
[ERROR] Vulkan error: VK_ERROR_INCOMPATIBLE_DRIVER
[FATAL] Cannot initialize graphics system
```

**Likely causes:**
- Missing Vulkan drivers
- Outdated graphics drivers
- Running in VM without GPU passthrough

**Fix:**
```bash
# Check Vulkan support
vulkaninfo | head -20

# Install/update Vulkan drivers
sudo apt update
sudo apt install vulkan-tools mesa-vulkan-drivers

# For NVIDIA (if applicable)
sudo apt install nvidia-vulkan-driver

# Test basic Vulkan
vkcube  # Should show spinning cube
```

#### 2. Permission/Runtime Issues
```bash
$ ./test_deployment.sh
[ERROR] Permission denied: /run/user/1000
[ERROR] Cannot create Wayland socket
[FATAL] Compositor initialization failed
```

**Fix:**
```bash
# Ensure proper runtime directory
sudo mkdir -p /run/user/$(id -u)
sudo chown $(id -u):$(id -g) /run/user/$(id -u)
export XDG_RUNTIME_DIR=/run/user/$(id -u)

# Alternative: use temp directory for testing
export XDG_RUNTIME_DIR=/tmp/wayland-test-$(id -u)
mkdir -p $XDG_RUNTIME_DIR
```

#### 3. Display/TTY Issues
```bash
$ ./test_deployment.sh
[ERROR] Cannot access DRM device
[ERROR] No displays found
[FATAL] No output devices available
```

**Likely causes:**
- Not running in proper TTY
- Display manager still running
- Missing permissions for graphics device

**Fix:**
```bash
# Switch to TTY first (Ctrl+Alt+F2)
# Stop display manager
sudo systemctl stop gdm3  # or sddm/lightdm

# Check graphics device permissions
ls -la /dev/dri/
# Should show card0, renderD128, etc.

# Add user to video group if needed
sudo usermod -a -G video $USER
# Then logout and login again
```

#### 4. Compositor Crash Scenarios
```bash
$ ./test_deployment.sh
[INFO] Compositor starting...
[INFO] Vulkan instance created successfully
[PANIC] thread 'main' panicked at 'called `Result::unwrap()` on an `Err` value'
Segmentation fault (core dumped)
```

**What this means:**
- Compositor hit an unhandled error
- Could be Vulkan issue, protocol bug, or memory corruption
- System should remain stable (you're in TTY)

### Recovery Strategies - When Things Go Wrong

#### 1. Immediate Recovery (Compositor Won't Start)
```bash
# You're still in TTY - system is safe
# Simply restart display manager to get back to normal desktop
sudo systemctl start gdm3

# Or reboot if needed
sudo reboot
```

#### 2. Compositor Crashes During Use
```bash
# If compositor crashes while running:
# 1. You'll be dropped back to TTY prompt
# 2. System is still stable
# 3. Restart display manager:
sudo systemctl start gdm3

# 4. Check logs for debugging:
journalctl -f  # Live log viewing
dmesg | tail   # Kernel messages
```

#### 3. System Appears Frozen
```bash
# If screen goes black or appears frozen:
# 1. Try switching TTYs: Ctrl+Alt+F1, F2, F3, etc.
# 2. If that works, you're in a recoverable state
# 3. Kill compositor from another TTY:
pkill custom-wayland-compositor
sudo systemctl start gdm3

# 4. If TTY switching doesn't work:
# Hold power button for 10 seconds (hard reboot)
# System will boot normally - no data loss risk
```

### Debugging Information

#### 1. Enable Debug Logging
```bash
# Run with verbose logging
RUST_LOG=debug ./target/release/custom-wayland-compositor

# Or specific modules
RUST_LOG=compositor_core=debug,vulkan_renderer=trace ./target/release/custom-wayland-compositor
```

#### 2. Monitor System Resources
```bash
# In another TTY, monitor system health
htop           # CPU/memory usage
nvidia-smi     # GPU usage (if NVIDIA)
dmesg -w       # Kernel messages
journalctl -f  # System logs
```

#### 3. Vulkan Debugging
```bash
# Enable Vulkan validation layers
export VK_INSTANCE_LAYERS=VK_LAYER_KHRONOS_validation
./target/release/custom-wayland-compositor

# This will show detailed Vulkan errors if any occur
```

### Progressive Testing Strategy

#### Stage 1: Minimal Test (Safest)
```bash
# Just start compositor, don't connect clients yet
./test_deployment.sh

# If it starts successfully, Ctrl+C to stop
# This validates basic initialization
```

#### Stage 2: Basic Client Test
```bash
# Start compositor in background
./target/release/custom-wayland-compositor &
COMPOSITOR_PID=$!

# Test basic client
export WAYLAND_DISPLAY=wayland-custom
cargo run --bin shm_basic_window

# Kill compositor
kill $COMPOSITOR_PID
```

#### Stage 3: Multiple Clients
```bash
# Test with multiple windows
cargo run --bin shm_basic_window &
cargo run --bin xdg_shell_interactions &
sleep 5
pkill -f "shm_basic_window|xdg_shell_interactions"
```

#### Stage 4: Real Applications
```bash
# Only after basic tests pass
weston-terminal &
# Try opening/closing, resizing, etc.
```

### Expected Performance Characteristics

#### 1. Resource Usage
```bash
# Normal expectations:
# CPU: 2-10% idle, 20-40% under load
# Memory: 50-200MB base usage
# GPU: Minimal idle, spikes during rendering

# Warning signs:
# CPU: >80% constant usage
# Memory: Growing continuously (memory leak)
# GPU: Constant high usage when idle
```

#### 2. Responsiveness
```bash
# Good signs:
# - Window movements feel smooth
# - Input responds immediately
# - Applications start quickly

# Warning signs:
# - Stuttering or jank during window operations
# - Input lag or missed events
# - Slow application startup
```

### Fallback Plan - Emergency Exit Strategy

#### 1. Quick Recovery Script
```bash
# Create emergency recovery script
tee ~/emergency_recovery.sh << EOF
#!/bin/bash
echo "=== Emergency Compositor Recovery ==="
pkill custom-wayland-compositor 2>/dev/null
sudo systemctl stop gdm3 2>/dev/null
sleep 2
sudo systemctl start gdm3
echo "Recovery complete - normal desktop should appear"
EOF

chmod +x ~/emergency_recovery.sh

# Run this if things go wrong:
# Ctrl+Alt+F2, login, ./emergency_recovery.sh
```

#### 2. Prevention Measures
```bash
# Before testing, ensure you have:
# 1. Another user account (backup login method)
# 2. SSH access enabled (remote recovery)
# 3. Recent backup of important work
# 4. Knowledge of basic TTY navigation

# Enable SSH for remote access
sudo systemctl enable ssh
sudo systemctl start ssh
```

### Success Metrics - How to Know It's Working

#### 1. Basic Success
- [ ] Compositor starts without errors
- [ ] Test clients connect and display windows
- [ ] Windows can be moved/resized
- [ ] Clean shutdown with Ctrl+C

#### 2. Good Performance
- [ ] Smooth 60fps window animations
- [ ] Immediate input response
- [ ] Low CPU/memory usage when idle
- [ ] No graphics artifacts or glitches

#### 3. Ready for Real Use
- [ ] Multiple applications run simultaneously
- [ ] Terminal emulator works properly
- [ ] Copy/paste functions
- [ ] System remains stable for extended periods

### The Bottom Line

**Your compositor is built on solid foundations** (Smithay + Vulkan), so the chances of causing any permanent damage are essentially zero. The worst-case scenario is:

1. Compositor doesn't start (easy fix - restart display manager)
2. Compositor crashes (drops to TTY - restart display manager) 
3. System appears frozen (hard reboot - no data loss)

**The risk is very low, and the potential learning is huge.** Your compositor has comprehensive protocol implementations and a robust graphics pipeline, so it's much more likely to work than not.

**Ready to give it a try?** The test deployment script provides a safe way to experiment, and you always have multiple escape routes back to your normal desktop.
