// Compositor Core - Main compositor logic and Wayland protocol handling
//
// This crate implements the core compositor functionality including:
// - Wayland server and protocol handling
// - Window management and layout
// - Input event processing
// - Integration with the Vulkan renderer

use compositor_utils::prelude::*;
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
pub use backend::Backend;

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
        info!("Initializing custom compositor");
        
        // Initialize renderer first
        let renderer = VulkanRenderer::new()
            .map_err(|e| CompositorError::init(format!("Failed to initialize renderer: {}", e)))?;
        
        info!("Renderer info: {:?}", renderer.get_info());
        
        // Initialize backend (DRM/libinput)
        let backend = Backend::new()
            .await
            .map_err(|e| CompositorError::init(format!("Failed to initialize backend: {}", e)))?;
        
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
            let _renderer = renderer; // Keep renderer for future use
            
            while running_clone.load(std::sync::atomic::Ordering::Relaxed) {
                // Process backend events (input, output changes, etc.)
                if let Err(e) = backend.process_events().await {
                    error!("Backend error: {}", e);
                    break;
                }
                
                // Render frame (placeholder for now)
                // TODO: Implement proper frame rendering
                
                // Yield to other tasks
                tokio::time::sleep(std::time::Duration::from_millis(16)).await; // ~60 FPS
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
}
