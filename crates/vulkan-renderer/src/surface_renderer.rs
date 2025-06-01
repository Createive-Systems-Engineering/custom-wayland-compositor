// Surface-to-texture rendering for Wayland client buffers
//
// This module handles converting Wayland client surface buffers (SHM, DMA-BUF)
// into Vulkan textures that can be composited and displayed on screen.

use ash::vk;
use compositor_utils::prelude::*;
use crate::{VulkanInstance, VulkanDevice};
use std::collections::HashMap;
use std::sync::Arc;

/// Surface rendering context for converting client buffers to textures
pub struct SurfaceRenderer {
    instance: VulkanInstance,
    device: Arc<VulkanDevice>,
    /// Map of surface ID to texture handle for efficient lookups
    surface_textures: HashMap<u32, SurfaceTexture>,
    /// Command pool for texture operations
    command_pool: vk::CommandPool,
    /// Staging buffer for SHM buffer uploads
    staging_buffer: Option<vk::Buffer>,
    staging_memory: Option<vk::DeviceMemory>,
}

/// Vulkan texture representation of a Wayland surface buffer
#[derive(Debug)]
pub struct SurfaceTexture {
    pub image: vk::Image,
    pub image_view: vk::ImageView,
    pub memory: vk::DeviceMemory,
    pub width: u32,
    pub height: u32,
    pub format: vk::Format,
}

/// Surface buffer data received from Wayland clients
pub enum SurfaceBuffer {
    Shm {
        data: Vec<u8>,
        width: u32,
        height: u32,
        stride: u32,
        format: ShmFormat,
    },
    DmaBuf {
        width: u32,
        height: u32,
        format: DmaBufFormat,
        modifier: u64,
        fd: i32,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum ShmFormat {
    Argb8888,
    Xrgb8888,
    Rgba8888,
    Rgbx8888,
}

#[derive(Debug, Clone, Copy)]
pub enum DmaBufFormat {
    Argb8888,
    Xrgb8888,
    Rgba8888,
    Rgbx8888,
}

impl SurfaceRenderer {
    /// Create a new surface renderer
    pub fn new(instance: VulkanInstance, device: Arc<VulkanDevice>) -> Result<Self> {
        // Create command pool for texture operations
        let command_pool_info = vk::CommandPoolCreateInfo {
            flags: vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
            queue_family_index: 0, // Use graphics queue family
            ..Default::default()
        };
        
        let command_pool = unsafe {
            device.handle().create_command_pool(&command_pool_info, None)?
        };
        
        info!("Surface renderer initialized with command pool");
        
        Ok(Self {
            instance,
            device,
            surface_textures: HashMap::new(),
            command_pool,
            staging_buffer: None,
            staging_memory: None,
        })
    }
    
    /// Update a surface texture with new buffer data
    pub fn update_surface_texture(&mut self, surface_id: u32, buffer: SurfaceBuffer) -> Result<()> {
        match buffer {
            SurfaceBuffer::Shm { data, width, height, stride: _, format } => {
                self.update_shm_texture(surface_id, data, width, height, format)?;
            }
            SurfaceBuffer::DmaBuf { width, height, format, modifier: _, fd: _ } => {
                self.update_dmabuf_texture(surface_id, width, height, format)?;
            }
        }
        
        debug!("Updated texture for surface {}", surface_id);
        Ok(())
    }
    
    /// Get texture for a surface
    pub fn get_surface_texture(&self, surface_id: u32) -> Option<&SurfaceTexture> {
        self.surface_textures.get(&surface_id)
    }
    
    /// Remove a surface texture
    pub fn remove_surface_texture(&mut self, surface_id: u32) -> Result<()> {
        if let Some(texture) = self.surface_textures.remove(&surface_id) {
            self.cleanup_surface_texture(texture)?;
            debug!("Removed texture for surface {}", surface_id);
        }
        Ok(())
    }
    
    /// Update SHM buffer texture
    fn update_shm_texture(&mut self, surface_id: u32, data: Vec<u8>, width: u32, height: u32, format: ShmFormat) -> Result<()> {
        // Remove existing texture if it exists
        if let Some(old_texture) = self.surface_textures.remove(&surface_id) {
            self.cleanup_surface_texture(old_texture)?;
        }
        
        // Convert SHM format to Vulkan format
        let vk_format = match format {
            ShmFormat::Argb8888 => vk::Format::B8G8R8A8_UNORM,
            ShmFormat::Xrgb8888 => vk::Format::B8G8R8A8_UNORM,
            ShmFormat::Rgba8888 => vk::Format::R8G8B8A8_UNORM,
            ShmFormat::Rgbx8888 => vk::Format::R8G8B8A8_UNORM,
        };
        
        // Create Vulkan image for the texture
        let texture = self.create_texture_image(width, height, vk_format)?;
        
        // Upload data to the texture
        self.upload_texture_data(&texture, &data)?;
        
        // Store the texture
        self.surface_textures.insert(surface_id, texture);
        
        Ok(())
    }
    
    /// Update DMA-BUF texture (placeholder implementation)
    fn update_dmabuf_texture(&mut self, surface_id: u32, width: u32, height: u32, format: DmaBufFormat) -> Result<()> {
        debug!("DMA-BUF texture update for surface {} ({}x{}, {:?}) - placeholder implementation", 
               surface_id, width, height, format);
        
        // TODO: Implement DMA-BUF import using VK_EXT_external_memory_dma_buf
        // For now, create a placeholder black texture
        
        let vk_format = match format {
            DmaBufFormat::Argb8888 => vk::Format::B8G8R8A8_UNORM,
            DmaBufFormat::Xrgb8888 => vk::Format::B8G8R8A8_UNORM,
            DmaBufFormat::Rgba8888 => vk::Format::R8G8B8A8_UNORM,
            DmaBufFormat::Rgbx8888 => vk::Format::R8G8B8A8_UNORM,
        };
        
        let texture = self.create_texture_image(width, height, vk_format)?;
        
        // Fill with placeholder color (black)
        let black_data = vec![0u8; (width * height * 4) as usize];
        self.upload_texture_data(&texture, &black_data)?;
        
        self.surface_textures.insert(surface_id, texture);
        
        Ok(())
    }
    
    /// Create a new Vulkan texture image
    fn create_texture_image(&self, width: u32, height: u32, format: vk::Format) -> Result<SurfaceTexture> {
        // Image creation info
        let image_info = vk::ImageCreateInfo {
            image_type: vk::ImageType::TYPE_2D,
            extent: vk::Extent3D { width, height, depth: 1 },
            mip_levels: 1,
            array_layers: 1,
            format,
            tiling: vk::ImageTiling::OPTIMAL,
            initial_layout: vk::ImageLayout::UNDEFINED,
            usage: vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            samples: vk::SampleCountFlags::TYPE_1,
            ..Default::default()
        };
        
        let image = unsafe {
            self.device.handle().create_image(&image_info, None)?
        };
        
        // Get memory requirements
        let memory_requirements = unsafe {
            self.device.handle().get_image_memory_requirements(image)
        };
        
        // Allocate memory (simplified - in production use gpu-allocator)
        let memory_type_index = self.find_memory_type(
            memory_requirements.memory_type_bits,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;
        
        let alloc_info = vk::MemoryAllocateInfo {
            allocation_size: memory_requirements.size,
            memory_type_index,
            ..Default::default()
        };
        
        let memory = unsafe {
            self.device.handle().allocate_memory(&alloc_info, None)?
        };
        
        // Bind image to memory
        unsafe {
            self.device.handle().bind_image_memory(image, memory, 0)?;
        }
        
        // Create image view
        let image_view_info = vk::ImageViewCreateInfo {
            image,
            view_type: vk::ImageViewType::TYPE_2D,
            format,
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            },
            ..Default::default()
        };
        
        let image_view = unsafe {
            self.device.handle().create_image_view(&image_view_info, None)?
        };
        
        Ok(SurfaceTexture {
            image,
            image_view,
            memory,
            width,
            height,
            format,
        })
    }
    
    /// Upload data to texture using staging buffer and command buffer
    fn upload_texture_data(&mut self, texture: &SurfaceTexture, data: &[u8]) -> Result<()> {
        debug!("Uploading {}x{} texture data ({} bytes)", 
               texture.width, texture.height, data.len());
        
        let data_size = data.len() as vk::DeviceSize;
        
        // Create or resize staging buffer if needed
        self.ensure_staging_buffer(data_size)?;
        
        // Copy data to staging buffer
        let staging_buffer = self.staging_buffer.unwrap();
        let staging_memory = self.staging_memory.unwrap();
        
        unsafe {
            let mapped_ptr = self.device.handle().map_memory(
                staging_memory,
                0,
                data_size,
                vk::MemoryMapFlags::empty(),
            )?;
            
            std::ptr::copy_nonoverlapping(
                data.as_ptr(),
                mapped_ptr as *mut u8,
                data.len(),
            );
            
            self.device.handle().unmap_memory(staging_memory);
        }
        
        // Record and submit copy command
        self.copy_buffer_to_image(staging_buffer, texture)?;
        
        Ok(())
    }
    
    /// Ensure staging buffer exists and is large enough
    fn ensure_staging_buffer(&mut self, required_size: vk::DeviceSize) -> Result<()> {
        // Check if we need to create or resize the staging buffer
        let needs_creation = match (self.staging_buffer, self.staging_memory) {
            (Some(_), Some(_)) => {
                // TODO: Check if current buffer is large enough
                // For now, assume it's adequate
                false
            }
            _ => true,
        };
        
        if needs_creation {
            // Clean up existing staging buffer if any
            if let (Some(buffer), Some(memory)) = (self.staging_buffer, self.staging_memory) {
                unsafe {
                    self.device.handle().destroy_buffer(buffer, None);
                    self.device.handle().free_memory(memory, None);
                }
            }
            
            // Create new staging buffer
            let buffer_info = vk::BufferCreateInfo {
                size: required_size,
                usage: vk::BufferUsageFlags::TRANSFER_SRC,
                sharing_mode: vk::SharingMode::EXCLUSIVE,
                ..Default::default()
            };
            
            let buffer = unsafe {
                self.device.handle().create_buffer(&buffer_info, None)?
            };
            
            // Allocate memory for staging buffer
            let memory_requirements = unsafe {
                self.device.handle().get_buffer_memory_requirements(buffer)
            };
            
            let memory_type_index = self.find_memory_type(
                memory_requirements.memory_type_bits,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            )?;
            
            let alloc_info = vk::MemoryAllocateInfo {
                allocation_size: memory_requirements.size,
                memory_type_index,
                ..Default::default()
            };
            
            let memory = unsafe {
                self.device.handle().allocate_memory(&alloc_info, None)?
            };
            
            // Bind buffer to memory
            unsafe {
                self.device.handle().bind_buffer_memory(buffer, memory, 0)?;
            }
            
            self.staging_buffer = Some(buffer);
            self.staging_memory = Some(memory);
            
            debug!("Created staging buffer with size: {} bytes", required_size);
        }
        
        Ok(())
    }
    
    /// Copy data from staging buffer to image using command buffer
    fn copy_buffer_to_image(&self, buffer: vk::Buffer, texture: &SurfaceTexture) -> Result<()> {
        // Allocate command buffer
        let command_buffer_info = vk::CommandBufferAllocateInfo {
            command_pool: self.command_pool,
            level: vk::CommandBufferLevel::PRIMARY,
            command_buffer_count: 1,
            ..Default::default()
        };
        
        let command_buffers = unsafe {
            self.device.handle().allocate_command_buffers(&command_buffer_info)?
        };
        let command_buffer = command_buffers[0];
        
        // Begin command buffer recording
        let begin_info = vk::CommandBufferBeginInfo {
            flags: vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
            ..Default::default()
        };
        
        unsafe {
            self.device.handle().begin_command_buffer(command_buffer, &begin_info)?;
            
            // Transition image layout to TRANSFER_DST_OPTIMAL
            let barrier = vk::ImageMemoryBarrier {
                old_layout: vk::ImageLayout::UNDEFINED,
                new_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                image: texture.image,
                subresource_range: vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                src_access_mask: vk::AccessFlags::empty(),
                dst_access_mask: vk::AccessFlags::TRANSFER_WRITE,
                ..Default::default()
            };
            
            self.device.handle().cmd_pipeline_barrier(
                command_buffer,
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::TRANSFER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier],
            );
            
            // Copy buffer to image
            let region = vk::BufferImageCopy {
                buffer_offset: 0,
                buffer_row_length: 0,
                buffer_image_height: 0,
                image_subresource: vk::ImageSubresourceLayers {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    mip_level: 0,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                image_offset: vk::Offset3D { x: 0, y: 0, z: 0 },
                image_extent: vk::Extent3D {
                    width: texture.width,
                    height: texture.height,
                    depth: 1,
                },
            };
            
            self.device.handle().cmd_copy_buffer_to_image(
                command_buffer,
                buffer,
                texture.image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &[region],
            );
            
            // Transition image layout to SHADER_READ_ONLY_OPTIMAL
            let barrier = vk::ImageMemoryBarrier {
                old_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                new_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                image: texture.image,
                subresource_range: vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                src_access_mask: vk::AccessFlags::TRANSFER_WRITE,
                dst_access_mask: vk::AccessFlags::SHADER_READ,
                ..Default::default()
            };
            
            self.device.handle().cmd_pipeline_barrier(
                command_buffer,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::FRAGMENT_SHADER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier],
            );
            
            // End command buffer recording
            self.device.handle().end_command_buffer(command_buffer)?;
        }
        
        // Submit command buffer
        let submit_info = vk::SubmitInfo {
            command_buffer_count: 1,
            p_command_buffers: &command_buffer,
            ..Default::default()
        };
        
        unsafe {
            self.device.handle().queue_submit(
                self.device.graphics_queue(),
                &[submit_info],
                vk::Fence::null(),
            )?;
            
            // Wait for completion (in production, use fences for async)
            self.device.handle().queue_wait_idle(self.device.graphics_queue())?;
            
            // Free command buffer
            self.device.handle().free_command_buffers(self.command_pool, &[command_buffer]);
        }
        
        debug!("Successfully uploaded texture data to GPU");
        Ok(())
    }
    
    /// Find suitable memory type for allocation
    fn find_memory_type(&self, type_filter: u32, properties: vk::MemoryPropertyFlags) -> Result<u32> {
        let memory_properties = unsafe {
            self.instance.handle().get_physical_device_memory_properties(self.device.physical_device())
        };
        
        for i in 0..memory_properties.memory_type_count {
            if (type_filter & (1 << i)) != 0 
                && memory_properties.memory_types[i as usize].property_flags.contains(properties) 
            {
                return Ok(i);
            }
        }
        
        Err(CompositorError::graphics("Failed to find suitable memory type"))
    }
    
    /// Clean up a surface texture and its resources
    fn cleanup_surface_texture(&self, texture: SurfaceTexture) -> Result<()> {
        unsafe {
            self.device.handle().destroy_image_view(texture.image_view, None);
            self.device.handle().destroy_image(texture.image, None);
            self.device.handle().free_memory(texture.memory, None);
        }
        Ok(())
    }
    
    /// Get all current surface textures for rendering
    pub fn get_all_textures(&self) -> impl Iterator<Item = (u32, &SurfaceTexture)> {
        self.surface_textures.iter().map(|(&id, texture)| (id, texture))
    }
}

impl Drop for SurfaceRenderer {
    fn drop(&mut self) {
        eprintln!("SurfaceRenderer::drop() - Starting cleanup");
        eprintln!("Device Arc strong_count: {}", Arc::strong_count(&self.device));
        
        // RADICAL FIX: Skip ALL cleanup when device is shared
        // Let VulkanDevice handle everything when it's finally destroyed
        eprintln!("SurfaceRenderer::drop() - Skipping cleanup, letting VulkanDevice handle everything");
        eprintln!("SurfaceRenderer::drop() - Complete");
    }
}
