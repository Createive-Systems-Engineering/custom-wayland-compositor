// Custom Wayland Compositor
// High-performance compositor built with Rust and Vulkan for 4K UI/UX development

use compositor_utils::prelude::*;
use compositor_utils::{log_startup_phase, log_error_with_context};
use compositor_core::Compositor;
use vulkan_renderer;
use std::env;
use std::time::Instant;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const AUTHORS: &str = env!("CARGO_PKG_AUTHORS");

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    
    // Handle command line arguments
    if args.len() > 1 {
        match args[1].as_str() {
            "--help" | "-h" => {
                print_help();
                return Ok(());
            }
            "--version" | "-v" => {
                print_version();
                return Ok(());
            }
            "--info" => {
                print_detailed_info();
                return Ok(());
            }
            "--check" => {
                check_system_requirements();
                return Ok(());
            }
            _ => {
                eprintln!("Unknown argument: {}", args[1]);
                eprintln!("Use --help for usage information");
                std::process::exit(1);
            }
        }
    }

    // Initialize logging system with enhanced diagnostics
    log_startup_phase("LOGGING_INIT", "Setting up comprehensive logging system");
    compositor_utils::setup_logging()?;
    
    log_startup_phase("COMPOSITOR_START", &format!("Starting Custom Wayland Compositor v{}", VERSION));
    info!("Target: 4K UI/UX development on Debian 12 Linux");
    
    // Print system information
    log_startup_phase("SYSTEM_INFO", "Gathering system information");
    print_system_info();
    
    // Create and run compositor with enhanced logging
    log_startup_phase("COMPOSITOR_INIT", "Initializing compositor with Vulkan renderer");
    let start_time = Instant::now();
    let compositor = Compositor::new().await
        .context("Failed to create compositor")?;
    let creation_time = start_time.elapsed();
    info!("Compositor creation completed in {:?}", creation_time);
    
    // Display connection information
    if let Some(socket_name) = compositor.wayland_socket_name() {
        log_startup_phase("WAYLAND_SOCKET", &format!("Socket created: {}", socket_name));
        info!("Wayland socket available: {}", socket_name);
        info!("Clients can connect with: WAYLAND_DISPLAY={}", socket_name);
    } else {
        warn!("No Wayland socket name available - this may indicate initialization issues");
    }
    
    // Execute startup script to launch essential applications (including VSCode)
    log_startup_phase("STARTUP_SCRIPT", "Executing application startup script");
    execute_startup_script().await;
    
    log_startup_phase("MAIN_LOOP", "Starting compositor main loop");
    info!("Compositor created successfully, starting main loop");
    
    // Run the compositor (this consumes self and handles its own cleanup)
    let start_time = Instant::now();
    if let Err(e) = compositor.run().await {
        log_error_with_context(&format!("{}", e), "Compositor Main Loop");
        error!("Compositor error: {}", e);
        return Err(e.into());
    }
    let run_time = start_time.elapsed();
    
    log_startup_phase("COMPOSITOR_SHUTDOWN", &format!("Compositor shut down successfully after running for {:?}", run_time));
    info!("Compositor shut down successfully");
    Ok(())
}

fn print_system_info() {
    info!("System Information:");
    
    // Get memory info
    let memory_stats = compositor_utils::memory::get_memory_stats();
    info!("  Memory - Current: {:.2}MB, Peak: {:.2}MB", 
          memory_stats.current_mb(), memory_stats.peak_mb());
    
    // Check for 4K display support
    if let Ok(display_env) = std::env::var("DISPLAY") {
        info!("  Display: {}", display_env);
    }
    
    if let Ok(wayland_display) = std::env::var("WAYLAND_DISPLAY") {
        info!("  Wayland Display: {}", wayland_display);
    }
    
    // Session information
    if let Ok(session_type) = std::env::var("XDG_SESSION_TYPE") {
        info!("  Session Type: {}", session_type);
    }
    
    // Current directory
    if let Ok(current_dir) = std::env::current_dir() {
        info!("  Working Directory: {}", current_dir.display());
    }
    
    // Check for required permissions
    check_permissions();
}

fn check_permissions() {
    // Check if we can access DRM devices
    let drm_paths = ["/dev/dri/card0", "/dev/dri/card1"];
    for path in &drm_paths {
        match std::fs::metadata(path) {
            Ok(_) => info!("  DRM device accessible: {}", path),
            Err(_) => warn!("  DRM device not accessible: {}", path),
        }
    }
    
    // Check for input device access
    match std::fs::read_dir("/dev/input") {
        Ok(entries) => {
            let count = entries.count();
            info!("  Input devices found: {}", count);
        }
        Err(e) => warn!("  Cannot access input devices: {}", e),
    }
}

fn print_help() {
    println!("Custom Wayland Compositor v{}", VERSION);
    println!("High-performance 4K compositor built with Rust and Vulkan");
    println!();
    println!("USAGE:");
    println!("    custom-wayland-compositor [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("    -h, --help      Show this help message and exit");
    println!("    -v, --version   Show version information and exit");
    println!("    --info          Show detailed system information and exit");
    println!("    --check         Check system requirements and exit");
    println!();
    println!("DESCRIPTION:");
    println!("    A next-generation Wayland compositor optimized for 4K displays and modern");
    println!("    UI/UX development. Features hardware-accelerated Vulkan rendering, advanced");
    println!("    protocol support, and glassmorphism/neomorphism effects.");
    println!();
    println!("    When started, the compositor creates a Wayland display socket that clients");
    println!("    can connect to. Use 'WAYLAND_DISPLAY=wayland-1 your-app' to run applications.");
    println!();
    println!("EXAMPLES:");
    println!("    # Start the compositor");
    println!("    custom-wayland-compositor");
    println!();
    println!("    # Check system requirements");
    println!("    custom-wayland-compositor --check");
    println!();
    println!("    # Run a Wayland application in the compositor");
    println!("    WAYLAND_DISPLAY=wayland-1 weston-terminal");
    println!();
    println!("REQUIREMENTS:");
    println!("    - Vulkan-capable GPU (NVIDIA, AMD, or Intel)");
    println!("    - Linux with DRM/KMS support");
    println!("    - User in 'video' and 'render' groups for full acceleration");
    println!("    - Wayland client applications for testing");
    println!();
    println!("For more information, visit: https://github.com/your-repo/custom-wayland-compositor");
}

fn print_version() {
    println!("Custom Wayland Compositor v{}", VERSION);
    println!("Built with Rust and Vulkan for 4K performance");
    if !AUTHORS.is_empty() {
        println!("Authors: {}", AUTHORS);
    }
}

fn print_detailed_info() {
    println!("Custom Wayland Compositor - System Information");
    println!("==============================================");
    
    // Initialize minimal logging for info display
    let _ = compositor_utils::setup_logging();
    
    print_system_info();
    
    // Additional technical details
    println!("\nVulkan Support:");
    match vulkan_renderer::VulkanRenderer::new() {
        Ok(renderer) => {
            let info = renderer.get_info();
            println!("  Device: {}", info.device_name);
            println!("  Vendor ID: 0x{:X}", info.vendor_id);
            println!("  Device Type: {}", info.device_type);
            println!("  API Version: {}.{}.{}", 
                     ash::vk::api_version_major(info.api_version),
                     ash::vk::api_version_minor(info.api_version),
                     ash::vk::api_version_patch(info.api_version));
            println!("  Status: Ready for 4K rendering");
        }
        Err(e) => {
            println!("  Status: Error - {}", e);
            println!("  This may indicate missing Vulkan drivers or permissions");
        }
    }
}

fn check_system_requirements() {
    println!("System Requirements Check");
    println!("========================");
    
    let mut all_good = true;
    
    // Check Vulkan support
    print!("Vulkan Support: ");
    match vulkan_renderer::VulkanRenderer::new() {
        Ok(_) => println!("[PASS] Vulkan renderer initialized successfully"),
        Err(e) => {
            println!("[FAIL] {}", e);
            all_good = false;
        }
    }
    
    // Check DRM access
    print!("DRM Device Access: ");
    let drm_paths = ["/dev/dri/card0", "/dev/dri/card1"];
    let drm_accessible = drm_paths.iter().any(|path| std::fs::metadata(path).is_ok());
    if drm_accessible {
        println!("[PASS] DRM devices accessible");
    } else {
        println!("[WARN] No DRM devices accessible - hardware acceleration may be limited");
    }
    
    // Check user groups
    print!("User Groups: ");
    if let Ok(output) = std::process::Command::new("groups").output() {
        let groups = String::from_utf8_lossy(&output.stdout);
        let has_video = groups.contains("video");
        let has_render = groups.contains("render");
        
        if has_video && has_render {
            println!("[PASS] User in video and render groups");
        } else {
            println!("[WARN] User not in video/render groups - run: sudo usermod -a -G video,render $USER");
        }
    } else {
        println!("[WARN] Could not check user groups");
    }
    
    // Check session type
    print!("Session Type: ");
    match std::env::var("XDG_SESSION_TYPE") {
        Ok(session_type) => {
            if session_type == "wayland" {
                println!("[PASS] Running in Wayland session");
            } else {
                println!("[INFO] Running in {} session - compositor will work but with limited privileges", session_type);
            }
        }
        Err(_) => println!("[WARN] Could not determine session type"),
    }
    
    // Summary
    println!();
    if all_good {
        println!("Summary: System is ready for 4K Wayland compositor!");
    } else {
        println!("Summary: System has some limitations but compositor should still work.");
        println!("See warnings above for optimization suggestions.");
    }
}

/// Execute the startup script to launch essential applications
async fn execute_startup_script() {
    let home_dir = match std::env::var("HOME") {
        Ok(home) => home,
        Err(_) => {
            warn!("Could not determine HOME directory, skipping startup script");
            return;
        }
    };
    
    let startup_script = format!("{}/.config/custom-compositor/startup.sh", home_dir);
    
    // Check if startup script exists
    if !std::path::Path::new(&startup_script).exists() {
        info!("No startup script found at {}, skipping", startup_script);
        return;
    }
    
    info!("Executing startup script: {}", startup_script);
    
    // Execute the startup script in the background
    match tokio::process::Command::new("bash")
        .arg(&startup_script)
        .spawn()
    {
        Ok(mut child) => {
            info!("Startup script launched successfully");
            
            // Don't wait for the script to complete - let it run in background
            tokio::spawn(async move {
                match child.wait().await {
                    Ok(status) => {
                        if status.success() {
                            info!("Startup script completed successfully");
                        } else {
                            warn!("Startup script exited with status: {}", status);
                        }
                    }
                    Err(e) => {
                        error!("Failed to wait for startup script: {}", e);
                    }
                }
            });
        }
        Err(e) => {
            error!("Failed to execute startup script: {}", e);
        }
    }
}
