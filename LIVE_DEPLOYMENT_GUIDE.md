# Live Deployment Guide
## Making the Custom Wayland Compositor Your Primary Display Server

### Phase 1: Safe Testing Environment

#### 1. Build and Test in TTY
```bash
# Build the compositor
cargo build --release

# Test in a TTY session (Ctrl+Alt+F2)
# Login to TTY2
sudo systemctl stop gdm  # or your current display manager
export XDG_RUNTIME_DIR=/run/user/$(id -u)
export WAYLAND_DISPLAY=wayland-1

# Start compositor in TTY
./target/release/custom-wayland-compositor
```

#### 2. Test with Basic Applications
```bash
# In another TTY (Ctrl+Alt+F3), test basic apps
export WAYLAND_DISPLAY=wayland-1
export XDG_RUNTIME_DIR=/run/user/$(id -u)

# Test with our test clients first
cargo run --bin shm_basic_window
cargo run --bin xdg_shell_interactions

# Test with real applications
weston-terminal  # or any Wayland-native terminal
firefox --wayland  # Firefox in Wayland mode
```

### Phase 2: Session Integration

#### 1. Create Desktop Session File
```bash
sudo tee /usr/share/wayland-sessions/custom-compositor.desktop << EOF
[Desktop Entry]
Name=Custom Wayland Compositor
Comment=High-performance 4K Wayland compositor
Exec=/home/shane/vscode/custom-wayland-compositor/target/release/custom-wayland-compositor
Type=Application
DesktopNames=CustomCompositor
EOF
```

#### 2. Install Compositor Binary
```bash
# Build release version
cargo build --release

# Install to system path
sudo cp target/release/custom-wayland-compositor /usr/local/bin/
sudo chmod +x /usr/local/bin/custom-wayland-compositor
```

#### 3. Configure Display Manager
```bash
# Update desktop session file to use system binary
sudo tee /usr/share/wayland-sessions/custom-compositor.desktop << EOF
[Desktop Entry]
Name=Custom Wayland Compositor
Comment=High-performance 4K Wayland compositor  
Exec=/usr/local/bin/custom-wayland-compositor
Type=Application
DesktopNames=CustomCompositor
EOF
```

### Phase 3: Essential Applications Setup

#### Terminal Emulator
Install a Wayland-native terminal:
```bash
sudo apt install weston-terminal alacritty kitty
```

#### Application Launcher
```bash
# Install wofi (Wayland dmenu replacement)
sudo apt install wofi

# Or build bemenu for Wayland
sudo apt install bemenu-wayland
```

#### Status Bar/Panel
```bash
# Install waybar (excellent Wayland status bar)
sudo apt install waybar

# Or use eww (widget system)
# This is where your custom app bar will eventually go!
```

### Phase 4: Production Deployment

#### 1. Backup Current Setup
```bash
# Backup current display manager config
sudo cp /etc/gdm3/daemon.conf /etc/gdm3/daemon.conf.backup
# or for SDDM: sudo cp /etc/sddm.conf /etc/sddm.conf.backup
```

#### 2. Set as Default Session
```bash
# Set default session for your user
echo "custom-compositor" > ~/.dmrc

# Or set system-wide default
sudo tee -a /etc/gdm3/daemon.conf << EOF

[daemon]
DefaultSession=custom-compositor
EOF
```

#### 3. Essential Environment Setup
Create startup script:
```bash
tee ~/.config/custom-compositor/startup.sh << EOF
#!/bin/bash

# Set up environment
export XDG_CURRENT_DESKTOP=CustomCompositor
export XDG_SESSION_DESKTOP=CustomCompositor
export QT_QPA_PLATFORM=wayland
export GDK_BACKEND=wayland

# Start essential services
waybar &
wofi --show drun &  # Application launcher

# Your app bar will start here eventually!
# /path/to/glassmorphic-app-bar &

# Start notification daemon
mako &

# Set wallpaper (if using a wallpaper setter)
# swaybg -i /path/to/wallpaper.jpg &

wait
EOF

chmod +x ~/.config/custom-compositor/startup.sh
```

### Phase 5: App Bar Development Environment

Once your compositor is running as your primary desktop:

#### 1. Create App Bar Project
```bash
# In a separate directory
mkdir -p ~/projects/glassmorphic-app-bar
cd ~/projects/glassmorphic-app-bar

# Initialize new Rust project
cargo init --name glassmorphic-app-bar

# Add Wayland client dependencies
cargo add wayland-client wayland-protocols smithay-client-toolkit
cargo add ash vulkan-loader  # For Vulkan rendering
cargo add tokio serde ron    # Async and config
```

#### 2. App Bar Development
Your app bar will be a **separate Wayland client** that:
- Connects to your compositor via Wayland protocols
- Uses `wlr_layer_shell` for always-on-top positioning
- Leverages your compositor's alpha blending for transparency
- Renders glassmorphic effects using Vulkan
- Communicates with compositor for special features

### Safety Features

#### Fallback Session
Always keep a backup session available:
```bash
# Install basic Wayland compositor as fallback
sudo apt install sway

# Keep GNOME/KDE as X11 fallback
# They'll still be available in the session selector
```

#### Emergency Recovery
```bash
# If compositor crashes, switch to TTY (Ctrl+Alt+F2)
sudo systemctl restart gdm

# Or kill compositor and restart display manager
sudo killall custom-wayland-compositor
sudo systemctl restart gdm
```

### Development Workflow

1. **Use your compositor daily** - This provides real-world testing
2. **Develop app bar separately** - Clean architecture, easier debugging  
3. **Iterate quickly** - Hot-reload compositor changes without losing work
4. **Test thoroughly** - Your daily use becomes continuous integration

### Key Advantages

✅ **Real-world validation** - Daily use exposes edge cases
✅ **Performance optimization** - Feel actual performance impact
✅ **Feature motivation** - Develop features you actually need
✅ **Clean separation** - App bar as client maintains good architecture
✅ **Professional workflow** - Using your own tools professionally

This approach transforms development from "toy project" to "production system development" - exactly what you need for a professional-grade compositor.
