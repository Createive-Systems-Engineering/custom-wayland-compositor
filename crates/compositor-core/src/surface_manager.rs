// Surface management system that bridges Wayland surfaces with Vulkan textures
//
// This module provides the interface between the Wayland server (which receives
// client surface data) and the Vulkan renderer (which renders textures to screen).

use compositor_utils::prelude::*;
use vulkan_renderer::{VulkanRenderer, SurfaceBuffer};
use wayland_server::protocol::wl_buffer::WlBuffer as WaylandBuffer;
use smithay::wayland::shm;
use smithay::wayland::dmabuf;
use smithay::backend::allocator::Buffer;
use drm_fourcc::DrmFourcc;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

/// Surface manager that coordinates between Wayland and Vulkan
pub struct SurfaceManager {
    renderer: Option<Arc<Mutex<VulkanRenderer>>>,
    /// Map of Wayland surface ID to our internal surface ID
    surface_mapping: HashMap<u64, u32>,
    next_surface_id: u32,
}

impl SurfaceManager {
    /// Create a new surface manager
    pub fn new() -> Self {
        info!("Initializing surface manager");
        
        Self {
            renderer: None,
            surface_mapping: HashMap::new(),
            next_surface_id: 1,
        }
    }
    
    /// Set the Vulkan renderer
    pub fn set_renderer(&mut self, renderer: Arc<Mutex<VulkanRenderer>>) {
        info!("Surface manager connected to Vulkan renderer");
        self.renderer = Some(renderer);
    }
    
    /// Register a new Wayland surface
    pub fn register_surface(&mut self, wayland_surface_id: u64) -> u32 {
        let surface_id = self.next_surface_id;
        self.next_surface_id += 1;
        
        self.surface_mapping.insert(wayland_surface_id, surface_id);
        
        info!("Registered surface: Wayland {} -> Internal {}", wayland_surface_id, surface_id);
        surface_id
    }
    
    /// Handle surface buffer commit from Wayland client
    pub fn handle_surface_commit(&mut self, wayland_surface_id: u64, buffer: &WaylandBuffer) -> Result<()> {
        let surface_id = match self.surface_mapping.get(&wayland_surface_id) {
            Some(&id) => id,
            None => {
                // Auto-register unknown surfaces
                self.register_surface(wayland_surface_id)
            }
        };
        
        // Convert Wayland buffer to our surface buffer format
        let surface_buffer = self.convert_wayland_buffer(buffer)?;
        
        // Update the renderer if available
        if let Some(ref renderer) = self.renderer {
            if let Ok(mut renderer) = renderer.lock() {
                // Extract buffer data and metadata for Vulkan renderer
                match &surface_buffer {
                    SurfaceBuffer::Shm { data, width, height, format, .. } => {
                        let vk_format = self.shm_format_to_vulkan(*format);
                        renderer.update_surface_buffer(surface_id, data, *width, *height, vk_format)?;
                    },
                    SurfaceBuffer::DmaBuf { width, height, format, .. } => {
                        // For DMA-BUF, we'll need to handle differently
                        // For now, create empty data as placeholder
                        let vk_format = self.dmabuf_format_to_vulkan(*format);
                        let empty_data = vec![0u8; (*width * *height * 4) as usize]; // 4 bytes per pixel
                        renderer.update_surface_buffer(surface_id, &empty_data, *width, *height, vk_format)?;
                    }
                }
                debug!("Updated surface {} with new buffer", surface_id);
            } else {
                warn!("Failed to lock renderer for surface update");
            }
        } else {
            debug!("No renderer available yet, surface buffer will be processed when renderer is connected");
        }
        
        Ok(())
    }
    
    /// Remove a surface
    pub fn remove_surface(&mut self, wayland_surface_id: u64) -> Result<()> {
        if let Some(surface_id) = self.surface_mapping.remove(&wayland_surface_id) {
            if let Some(ref renderer) = self.renderer {
                if let Ok(mut renderer) = renderer.lock() {
                    renderer.remove_surface(surface_id)?;
                    info!("Removed surface: Wayland {} -> Internal {}", wayland_surface_id, surface_id);
                }
            }
        }
        Ok(())
    }
    
    /// Convert Wayland buffer to our surface buffer format
    fn convert_wayland_buffer(&self, buffer: &WaylandBuffer) -> Result<SurfaceBuffer> {
        // Try to handle as DMA-BUF first
        if let Ok(dmabuf) = dmabuf::get_dmabuf(buffer) {
            debug!("Converting DMA-BUF: {}x{}, format: {:?}", 
                   dmabuf.width(), dmabuf.height(), dmabuf.format());
            
            let format = match dmabuf.format().code {
                // Common formats - map to our enum
                DrmFourcc::Argb8888 => vulkan_renderer::surface_renderer::DmaBufFormat::Argb8888,
                DrmFourcc::Xrgb8888 => vulkan_renderer::surface_renderer::DmaBufFormat::Xrgb8888,
                DrmFourcc::Rgba8888 => vulkan_renderer::surface_renderer::DmaBufFormat::Rgba8888,
                DrmFourcc::Rgbx8888 => vulkan_renderer::surface_renderer::DmaBufFormat::Rgbx8888,
                _ => {
                    warn!("Unsupported DMA-BUF format: {:?}", dmabuf.format());
                    vulkan_renderer::surface_renderer::DmaBufFormat::Argb8888 // Fallback
                }
            };
            
            // Get the first plane FD for basic handling
            // Note: Using placeholder FD as we need actual dmabuf integration
            let fd = -1; // TODO: Implement proper dmabuf FD extraction
            
            return Ok(SurfaceBuffer::DmaBuf {
                width: dmabuf.width(),
                height: dmabuf.height(),
                format,
                modifier: dmabuf.format().modifier.into(),
                fd,
            });
        }
        
        // Try to handle as SHM buffer
        if let Ok((data, shm_attributes)) = shm::with_buffer_contents(buffer, |ptr, len, data| {
            let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
            (slice.to_vec(), data.clone())
        }) {
            debug!("Converting SHM buffer: {}x{}, format: {:?}", 
                   shm_attributes.width, shm_attributes.height, shm_attributes.format);
            
            let format = match shm_attributes.format {
                wayland_server::protocol::wl_shm::Format::Argb8888 => 
                    vulkan_renderer::surface_renderer::ShmFormat::Argb8888,
                wayland_server::protocol::wl_shm::Format::Xrgb8888 => 
                    vulkan_renderer::surface_renderer::ShmFormat::Xrgb8888,
                wayland_server::protocol::wl_shm::Format::Rgba8888 => 
                    vulkan_renderer::surface_renderer::ShmFormat::Rgba8888,
                wayland_server::protocol::wl_shm::Format::Rgbx8888 => 
                    vulkan_renderer::surface_renderer::ShmFormat::Rgbx8888,
                _ => {
                    warn!("Unsupported SHM format: {:?}", shm_attributes.format);
                    vulkan_renderer::surface_renderer::ShmFormat::Argb8888 // Fallback
                }
            };
            
            return Ok(SurfaceBuffer::Shm {
                data,
                width: shm_attributes.width as u32,
                height: shm_attributes.height as u32,
                stride: shm_attributes.stride as u32,
                format,
            });
        }
        
        Err(CompositorError::wayland("Unknown buffer type - not SHM or DMA-BUF"))
    }
    
    /// Get number of active surfaces
    pub fn surface_count(&self) -> usize {
        self.surface_mapping.len()
    }
    
    /// Convert SHM format to Vulkan format
    fn shm_format_to_vulkan(&self, format: vulkan_renderer::surface_renderer::ShmFormat) -> ash::vk::Format {
        match format {
            vulkan_renderer::surface_renderer::ShmFormat::Argb8888 => ash::vk::Format::B8G8R8A8_UNORM,
            vulkan_renderer::surface_renderer::ShmFormat::Xrgb8888 => ash::vk::Format::B8G8R8A8_UNORM,
            vulkan_renderer::surface_renderer::ShmFormat::Rgba8888 => ash::vk::Format::R8G8B8A8_UNORM,
            vulkan_renderer::surface_renderer::ShmFormat::Rgbx8888 => ash::vk::Format::R8G8B8A8_UNORM,
        }
    }
    
    /// Convert DMA-BUF format to Vulkan format
    fn dmabuf_format_to_vulkan(&self, format: vulkan_renderer::surface_renderer::DmaBufFormat) -> ash::vk::Format {
        match format {
            vulkan_renderer::surface_renderer::DmaBufFormat::Argb8888 => ash::vk::Format::B8G8R8A8_UNORM,
            vulkan_renderer::surface_renderer::DmaBufFormat::Xrgb8888 => ash::vk::Format::B8G8R8A8_UNORM,
            vulkan_renderer::surface_renderer::DmaBufFormat::Rgba8888 => ash::vk::Format::R8G8B8A8_UNORM,
            vulkan_renderer::surface_renderer::DmaBufFormat::Rgbx8888 => ash::vk::Format::R8G8B8A8_UNORM,
        }
    }
}

impl Drop for SurfaceManager {
    fn drop(&mut self) {
        info!("Surface manager shutting down with {} active surfaces", self.surface_count());
        
        // Clean up all surfaces
        let wayland_ids: Vec<u64> = self.surface_mapping.keys().cloned().collect();
        for wayland_id in wayland_ids {
            if let Err(e) = self.remove_surface(wayland_id) {
                error!("Failed to cleanup surface {}: {}", wayland_id, e);
            }
        }
    }
}
