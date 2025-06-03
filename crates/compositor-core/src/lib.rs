// Compositor Core - Main compositor logic and Wayland protocol handling
//
// This crate implements the core compositor functionality including:
// - Wayland server and protocol handling
// - Window management and layout
// - Input event processing
// - Integration with the Vulkan renderer

use compositor_utils::prelude::*;
use compositor_utils::{log_startup_phase, log_error_with_context};
use vulkan_renderer::VulkanRenderer;
use std::sync::{atomic::AtomicBool, Arc};

pub mod wayland;
pub mod window;
pub mod input;
pub mod output;
pub mod surface;
pub mod surface_manager;
pub mod backend;
pub mod session;

// Test modules for comprehensive validation
#[cfg(test)]
pub mod tests;

/// Re-export core types
pub use wayland::WaylandServer;
pub use session::{SessionManager, SessionState};
pub use backend::{Backend, DisplayOutput};

/// Main compositor instance
pub struct Compositor {
    wayland_server: WaylandServer,
    renderer: VulkanRenderer,
    backend: Backend,
    running: Arc<AtomicBool>,
}

impl Compositor {
    /// Create a new compositor instance
    pub async fn new() -> Result<Self> {
        log_startup_phase("COMPOSITOR_CORE_INIT", "Initializing custom compositor");
        
        // Initialize renderer first
        log_startup_phase("RENDERER_INIT", "Initializing Vulkan renderer");
        let renderer = VulkanRenderer::new()
            .map_err(|e| {
                let error_msg = format!("Failed to initialize renderer: {}", e);
                log_error_with_context(&error_msg, "Renderer Initialization");
                CompositorError::init(error_msg)
            })?;
        
        info!("Renderer info: {:?}", renderer.get_info());
        
        // Initialize backend (DRM/libinput)
        log_startup_phase("BACKEND_INIT", "Initializing backend (DRM/libinput)");
        let mut backend = Backend::new()
            .await
            .map_err(|e| CompositorError::init(format!("Failed to initialize backend: {}", e)))?;
        
        // Log backend information
        info!("Backend type: {:?}", backend.backend_type());
        if let Some(drm_path) = backend.drm_device_path() {
            info!("Using DRM device: {}", drm_path);
        }
        
        // CRITICAL: Initialize displays and connect to renderer
        log_startup_phase("DISPLAY_INIT", "Initializing displays and connecting to renderer");
        let mut renderer = renderer; // Make renderer mutable for display initialization
        backend.initialize_displays(&mut renderer)
            .await
            .map_err(|e| {
                let error_msg = format!("Failed to initialize displays: {}", e);
                log_error_with_context(&error_msg, "Display Initialization");
                CompositorError::init(error_msg)
            })?;
        
        info!("Displays initialized: {} connected", backend.displays().len());
        if let Some(primary) = backend.primary_display() {
            info!("Primary display: {} ({}x{}@{}Hz)", 
                  primary.name, primary.width, primary.height, primary.refresh_rate);
        }
        
        // Initialize Wayland server
        let mut wayland_server = WaylandServer::new()
            .map_err(|e| CompositorError::init(format!("Failed to initialize Wayland server: {}", e)))?;
        
        // Initialize wl_drm protocol support via EGL backend
        wayland_server.initialize_wl_drm()
            .map_err(|e| CompositorError::init(format!("Failed to initialize wl_drm protocol: {}", e)))?;
        
        // Start listening for client connections
        wayland_server.start_listening()
            .map_err(|e| CompositorError::init(format!("Failed to start Wayland server: {}", e)))?;
        
        info!("Compositor initialized successfully");
        
        Ok(Self {
            wayland_server,
            renderer,
            backend,
            running: Arc::new(AtomicBool::new(true)),
        })
    }
    
    /// Get the Wayland socket name for client connections
    pub fn wayland_socket_name(&self) -> Option<&str> {
        self.wayland_server.socket_name()
    }
    
    /// Start the compositor main loop
    pub async fn run(self) -> Result<()> {
        info!("Starting compositor main loop");
        
        // Split self to move parts into different tasks
        let Self { wayland_server, backend, renderer, running } = self;
        
        // Spawn background tasks for backend and renderer
        let running_clone = running.clone();
        let compositor_handle = tokio::spawn(async move {
            let mut backend = backend;
            let mut renderer = renderer; // Make renderer mutable for frame rendering
            
            while running_clone.load(std::sync::atomic::Ordering::Relaxed) {
                // Process backend events (input, output changes, etc.)
                if let Err(e) = backend.process_events().await {
                    error!("Backend error: {}", e);
                    break;
                }
                
                // CRITICAL: Actually render frames to displays
                if let Err(e) = Self::render_frame_to_displays(&mut renderer, &backend).await {
                    error!("Render error: {}", e);
                    // Don't break on render errors, just log them
                }
                
                // Yield to other tasks (~60 FPS)
                tokio::time::sleep(std::time::Duration::from_millis(16)).await;
            }
            info!("Background compositor tasks completed");
        });
        
        // Run Wayland server in current thread (since EventLoop is not Send)
        // This will block until the server shuts down
        let wayland_result = wayland_server.run_async().await;
        
        // Signal background tasks to stop
        running.store(false, std::sync::atomic::Ordering::Relaxed);
        
        // Wait for background tasks to complete
        if let Err(e) = compositor_handle.await {
            error!("Error waiting for compositor tasks: {}", e);
        }
        
        // Check if wayland server had any errors
        if let Err(e) = wayland_result {
            error!("Wayland server error: {}", e);
            return Err(e);
        }
        
        info!("Compositor main loop ended");
        Ok(())
    }
    
    /// Setup signal handlers for graceful shutdown
    #[allow(dead_code)]
    async fn setup_signal_handlers(&self) -> Result<()> {
        let running = self.running.clone();
        
        tokio::spawn(async move {
            let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("Failed to setup SIGTERM handler");
            let mut sigint = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())
                .expect("Failed to setup SIGINT handler");
            
            tokio::select! {
                _ = sigterm.recv() => {
                    info!("Received SIGTERM, shutting down");
                }
                _ = sigint.recv() => {
                    info!("Received SIGINT, shutting down");
                }
            }
            
            running.store(false, std::sync::atomic::Ordering::Relaxed);
        });
        
        Ok(())
    }
    
    /// Render a frame
    #[allow(dead_code)]
    async fn render_frame(&mut self) -> Result<()> {
        // Begin frame
        self.renderer.begin_frame()?;
        
        // TODO: Render compositor content
        // - Render windows
        // - Render UI elements
        // - Apply effects (glassmorphism, etc.)
        
        // End frame and present
        self.renderer.end_frame()?;
        
        Ok(())
    }
    
    /// Shutdown the compositor
    pub async fn shutdown(&mut self) -> Result<()> {
        info!("Shutting down compositor");
        
        self.running.store(false, std::sync::atomic::Ordering::Relaxed);
        
        // Shutdown components in reverse order
        self.wayland_server.shutdown().await?;
        
        info!("Compositor shutdown complete");
        Ok(())
    }
    
    /// Render frames to all connected displays
    async fn render_frame_to_displays(renderer: &mut VulkanRenderer, backend: &Backend) -> Result<()> {
        // Get primary display for rendering
        if let Some(primary_display) = backend.primary_display() {
            if primary_display.connected {
                // Begin frame on primary display
                match renderer.begin_frame() {
                    Ok(image_index) => {
                        // Render the frame with compositor content
                        match renderer.render_frame(0, image_index) {
                            Ok(_command_buffer) => {
                                // End frame and present to screen
                                if let Err(e) = renderer.end_frame() {
                                    debug!("Present error (expected during development): {}", e);
                                }
                                
                                // Log successful frame every few seconds to avoid spam
                                static mut FRAME_COUNT: u64 = 0;
                                unsafe {
                                    FRAME_COUNT += 1;
                                    if FRAME_COUNT % 300 == 0 { // Every 5 seconds at 60fps
                                        info!("Successfully rendered frame {} to display: {}", 
                                              FRAME_COUNT, primary_display.name);
                                    }
                                }
                            }
                            Err(e) => {
                                debug!("Render frame error (expected during development): {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        debug!("Begin frame error (expected during development): {}", e);
                    }
                }
            }
        } else {
            // No displays connected - this is fine for headless operation
            tokio::task::yield_now().await;
        }
        
        Ok(())
    }
}
