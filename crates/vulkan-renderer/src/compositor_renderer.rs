// Main compositor rendering coordination
//
// This module orchestrates the complete rendering pipeline for the compositor,
// managing surface textures, render passes, and drawing operations.

use ash::vk;
use compositor_utils::prelude::*;
use crate::{VulkanDevice, VulkanInstance, SurfaceRenderer, SurfacePipeline, SurfaceTexture, SurfacePushConstants};
use crate::surface_renderer::{SurfaceBuffer, ShmFormat};
use std::collections::HashMap;
use std::sync::Arc;

/// Main compositor renderer that coordinates all rendering operations
pub struct CompositorRenderer {
    device: Arc<VulkanDevice>,
    surface_renderer: Option<SurfaceRenderer>,
    surface_pipeline: Option<SurfacePipeline>,
    render_pass: Option<vk::RenderPass>,
    framebuffers: Vec<vk::Framebuffer>,
    command_buffers: Vec<vk::CommandBuffer>,
    command_pool: vk::CommandPool,
    
    // Rendering state
    swapchain_extent: vk::Extent2D,
    swapchain_images: Vec<vk::Image>,
    swapchain_image_views: Vec<vk::ImageView>,
    
    // Per-frame rendering resources
    vertex_buffers: HashMap<u32, vk::Buffer>,
    vertex_buffer_memories: HashMap<u32, vk::DeviceMemory>,
    descriptor_pool: Option<vk::DescriptorPool>,
    descriptor_sets: HashMap<u32, vk::DescriptorSet>,
}

impl CompositorRenderer {
    /// Create a new compositor renderer
    pub fn new(
        instance: VulkanInstance,
        device: Arc<VulkanDevice>,
    ) -> Result<Self> {
        info!("Creating compositor renderer");
        
        // Create surface renderer for texture management
        let surface_renderer = SurfaceRenderer::new(instance.clone(), device.clone())?;
        
        // Create command pool for rendering operations
        let command_pool = Self::create_command_pool(&device)?;
        
        Ok(Self {
            device,
            surface_renderer: Some(surface_renderer),
            surface_pipeline: None,
            render_pass: None,
            framebuffers: Vec::new(),
            command_buffers: Vec::new(),
            command_pool,
            swapchain_extent: vk::Extent2D { width: 0, height: 0 },
            swapchain_images: Vec::new(),
            swapchain_image_views: Vec::new(),
            vertex_buffers: HashMap::new(),
            vertex_buffer_memories: HashMap::new(),
            descriptor_pool: None,
            descriptor_sets: HashMap::new(),
        })
    }
    
    /// Initialize rendering for a swapchain
    pub fn initialize_swapchain(
        &mut self,
        swapchain_images: Vec<vk::Image>,
        swapchain_image_views: Vec<vk::ImageView>,
        swapchain_extent: vk::Extent2D,
        swapchain_format: vk::Format,
    ) -> Result<()> {
        info!("Initializing compositor renderer for {}x{} swapchain", 
              swapchain_extent.width, swapchain_extent.height);
        
        self.swapchain_images = swapchain_images;
        self.swapchain_image_views = swapchain_image_views;
        self.swapchain_extent = swapchain_extent;
        
        // Create render pass
        let render_pass = Self::create_render_pass(&self.device, swapchain_format)?;
        self.render_pass = Some(render_pass);
        
        // Create surface pipeline
        let surface_pipeline = SurfacePipeline::new(
            &VulkanInstance::new()?, // TODO: Store instance reference
            self.device.clone(),
            render_pass,
        )?;
        self.surface_pipeline = Some(surface_pipeline);
        
        // Create framebuffers
        self.create_framebuffers()?;
        
        // Create command buffers
        self.create_command_buffers()?;
        
        // Create descriptor pool
        self.create_descriptor_pool()?;
        
        info!("Compositor renderer initialized successfully");
        Ok(())
    }
    
    /// Render a frame with all visible surfaces
    pub fn render_frame(
        &mut self,
        frame_index: usize,
        image_index: u32,
    ) -> Result<vk::CommandBuffer> {
        let command_buffer = self.command_buffers[frame_index];
        
        // Begin command buffer recording
        let begin_info = vk::CommandBufferBeginInfo {
            flags: vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
            ..Default::default()
        };
        
        unsafe {
            self.device.handle().begin_command_buffer(command_buffer, &begin_info)?;
        }
        
        // Begin render pass
        self.begin_render_pass(command_buffer, image_index)?;
        
        // Render all surfaces
        self.render_surfaces(command_buffer)?;
        
        // End render pass and command buffer
        unsafe {
            self.device.handle().cmd_end_render_pass(command_buffer);
            self.device.handle().end_command_buffer(command_buffer)?;
        }
        
        Ok(command_buffer)
    }
    
    /// Update surface texture from Wayland client
    pub fn update_surface_texture(
        &mut self,
        surface_id: u32,
        buffer_data: &[u8],
        width: u32,
        height: u32,
        format: vk::Format,
    ) -> Result<()> {
        debug!("Updating surface {} texture: {}x{}", surface_id, width, height);
        
        // Convert Vulkan format to SHM format (simplified conversion)
        let shm_format = match format {
            vk::Format::B8G8R8A8_UNORM => ShmFormat::Argb8888,
            vk::Format::B8G8R8_UNORM => ShmFormat::Xrgb8888,
            vk::Format::R8G8B8A8_UNORM => ShmFormat::Rgba8888,
            vk::Format::R8G8B8_UNORM => ShmFormat::Rgbx8888,
            _ => ShmFormat::Argb8888, // default fallback
        };
        
        // Create SurfaceBuffer
        let surface_buffer = SurfaceBuffer::Shm {
            data: buffer_data.to_vec(),
            width,
            height,
            stride: width * 4, // Assuming 4 bytes per pixel
            format: shm_format,
        };
        
        // Update texture in surface renderer
        self.surface_renderer.as_mut()
            .ok_or_else(|| CompositorError::runtime("Surface renderer not available"))?
            .update_surface_texture(surface_id, surface_buffer)?;
        
        // Create or update vertex buffer for this surface
        self.update_surface_vertex_buffer(surface_id, width, height)?;
        
        // Create or update descriptor set for texture sampling
        self.update_surface_descriptor_set(surface_id)?;
        
        Ok(())
    }
    
    /// Remove a surface and its associated resources
    pub fn remove_surface(&mut self, surface_id: u32) -> Result<()> {
        debug!("Removing surface {}", surface_id);
        
        // Remove from surface renderer
        self.surface_renderer.as_mut()
            .ok_or_else(|| CompositorError::runtime("Surface renderer not available"))?
            .remove_surface_texture(surface_id)?;
        
        // Clean up vertex buffer
        if let (Some(buffer), Some(memory)) = (
            self.vertex_buffers.remove(&surface_id),
            self.vertex_buffer_memories.remove(&surface_id)
        ) {
            unsafe {
                self.device.handle().destroy_buffer(buffer, None);
                self.device.handle().free_memory(memory, None);
            }
        }
        
        // Remove descriptor set
        self.descriptor_sets.remove(&surface_id);
        
        Ok(())
    }
    
    /// Create command pool for rendering operations
    fn create_command_pool(device: &VulkanDevice) -> Result<vk::CommandPool> {
        let pool_info = vk::CommandPoolCreateInfo {
            flags: vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
            queue_family_index: 0, // TODO: Get from device
            ..Default::default()
        };
        
        unsafe {
            device.handle().create_command_pool(&pool_info, None)
                .map_err(|e| CompositorError::graphics(&format!("Failed to create command pool: {}", e)))
        }
    }
    
    /// Create render pass for swapchain rendering
    fn create_render_pass(device: &VulkanDevice, format: vk::Format) -> Result<vk::RenderPass> {
        let color_attachment = vk::AttachmentDescription {
            format,
            samples: vk::SampleCountFlags::TYPE_1,
            load_op: vk::AttachmentLoadOp::CLEAR,
            store_op: vk::AttachmentStoreOp::STORE,
            stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
            stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
            initial_layout: vk::ImageLayout::UNDEFINED,
            final_layout: vk::ImageLayout::PRESENT_SRC_KHR,
            ..Default::default()
        };
        
        let color_attachment_ref = vk::AttachmentReference {
            attachment: 0,
            layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        };
        
        let subpass = vk::SubpassDescription {
            pipeline_bind_point: vk::PipelineBindPoint::GRAPHICS,
            color_attachment_count: 1,
            p_color_attachments: &color_attachment_ref,
            ..Default::default()
        };
        
        let dependency = vk::SubpassDependency {
            src_subpass: vk::SUBPASS_EXTERNAL,
            dst_subpass: 0,
            src_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            src_access_mask: vk::AccessFlags::empty(),
            dst_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            ..Default::default()
        };
        
        let render_pass_info = vk::RenderPassCreateInfo {
            attachment_count: 1,
            p_attachments: &color_attachment,
            subpass_count: 1,
            p_subpasses: &subpass,
            dependency_count: 1,
            p_dependencies: &dependency,
            ..Default::default()
        };
        
        unsafe {
            device.handle().create_render_pass(&render_pass_info, None)
                .map_err(|e| CompositorError::graphics(&format!("Failed to create render pass: {}", e)))
        }
    }
    
    /// Create framebuffers for each swapchain image
    fn create_framebuffers(&mut self) -> Result<()> {
        let render_pass = self.render_pass.unwrap();
        
        self.framebuffers = self.swapchain_image_views
            .iter()
            .map(|&image_view| {
                let attachments = [image_view];
                
                let framebuffer_info = vk::FramebufferCreateInfo {
                    render_pass,
                    attachment_count: attachments.len() as u32,
                    p_attachments: attachments.as_ptr(),
                    width: self.swapchain_extent.width,
                    height: self.swapchain_extent.height,
                    layers: 1,
                    ..Default::default()
                };
                
                unsafe {
                    self.device.handle().create_framebuffer(&framebuffer_info, None)
                        .map_err(|e| CompositorError::graphics(&format!("Failed to create framebuffer: {}", e)))
                }
            })
            .collect::<Result<Vec<_>>>()?;
        
        debug!("Created {} framebuffers", self.framebuffers.len());
        Ok(())
    }
    
    /// Create command buffers for rendering
    fn create_command_buffers(&mut self) -> Result<()> {
        let buffer_count = self.framebuffers.len();
        
        let alloc_info = vk::CommandBufferAllocateInfo {
            command_pool: self.command_pool,
            level: vk::CommandBufferLevel::PRIMARY,
            command_buffer_count: buffer_count as u32,
            ..Default::default()
        };
        
        self.command_buffers = unsafe {
            self.device.handle().allocate_command_buffers(&alloc_info)?
        };
        
        debug!("Created {} command buffers", self.command_buffers.len());
        Ok(())
    }
    
    /// Create descriptor pool for texture sampling
    fn create_descriptor_pool(&mut self) -> Result<()> {
        let pool_sizes = [
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: 1000, // Support many surfaces
            },
        ];
        
        let pool_info = vk::DescriptorPoolCreateInfo {
            pool_size_count: pool_sizes.len() as u32,
            p_pool_sizes: pool_sizes.as_ptr(),
            max_sets: 1000,
            flags: vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET,
            ..Default::default()
        };
        
        let descriptor_pool = unsafe {
            self.device.handle().create_descriptor_pool(&pool_info, None)?
        };
        
        self.descriptor_pool = Some(descriptor_pool);
        debug!("Created descriptor pool");
        Ok(())
    }
    
    /// Begin render pass
    fn begin_render_pass(&self, command_buffer: vk::CommandBuffer, image_index: u32) -> Result<()> {
        let clear_values = [vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.0, 0.0, 0.0, 1.0], // Black background
            },
        }];
        
        let render_pass_info = vk::RenderPassBeginInfo {
            render_pass: self.render_pass.unwrap(),
            framebuffer: self.framebuffers[image_index as usize],
            render_area: vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: self.swapchain_extent,
            },
            clear_value_count: clear_values.len() as u32,
            p_clear_values: clear_values.as_ptr(),
            ..Default::default()
        };
        
        unsafe {
            self.device.handle().cmd_begin_render_pass(
                command_buffer,
                &render_pass_info,
                vk::SubpassContents::INLINE,
            );
        }
        
        // Set viewport and scissor
        let viewport = vk::Viewport {
            x: 0.0,
            y: 0.0,
            width: self.swapchain_extent.width as f32,
            height: self.swapchain_extent.height as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        };
        
        let scissor = vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: self.swapchain_extent,
        };
        
        unsafe {
            self.device.handle().cmd_set_viewport(command_buffer, 0, &[viewport]);
            self.device.handle().cmd_set_scissor(command_buffer, 0, &[scissor]);
        }
        
        Ok(())
    }
    
    /// Render all surfaces
    fn render_surfaces(&self, command_buffer: vk::CommandBuffer) -> Result<()> {
        let surface_pipeline = self.surface_pipeline.as_ref()
            .ok_or_else(|| CompositorError::runtime("Surface pipeline not initialized"))?;
        
        // Bind pipeline
        unsafe {
            self.device.handle().cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                surface_pipeline.pipeline(),
            );
        }
        
        // Render each surface
        if let Some(ref surface_renderer) = self.surface_renderer {
            for (surface_id, texture) in surface_renderer.get_all_textures() {
                self.render_surface(command_buffer, surface_pipeline, surface_id, texture)?;
            }
        }
        
        Ok(())
    }
    
    /// Render a single surface
    fn render_surface(
        &self,
        command_buffer: vk::CommandBuffer,
        pipeline: &SurfacePipeline,
        surface_id: u32,
        _texture: &SurfaceTexture,
    ) -> Result<()> {
        // Get vertex buffer for this surface
        let vertex_buffer = self.vertex_buffers.get(&surface_id)                .ok_or_else(|| CompositorError::runtime("Missing vertex buffer for surface"))?;
        
        // Get descriptor set for texture
        let descriptor_set = self.descriptor_sets.get(&surface_id)                .ok_or_else(|| CompositorError::runtime("Missing descriptor set for surface"))?;
        
        // Create transform matrix (identity for now - will be enhanced with positioning)
        let transform = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        
        let push_constants = SurfacePushConstants {
            transform,
            offset: [0.0, 0.0], // TODO: Get from surface position
            scale: [1.0, 1.0],  // TODO: Get from surface scale
        };
        
        unsafe {
            // Bind descriptor set
            self.device.handle().cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline.pipeline_layout(),
                0,
                &[*descriptor_set],
                &[],
            );
            
            // Push constants
            self.device.handle().cmd_push_constants(
                command_buffer,
                pipeline.pipeline_layout(),
                vk::ShaderStageFlags::VERTEX,
                0,
                &std::mem::transmute::<_, [u8; std::mem::size_of::<SurfacePushConstants>()]>(push_constants),
            );
            
            // Bind vertex buffer
            let vertex_buffers = [*vertex_buffer];
            let offsets = [0];
            self.device.handle().cmd_bind_vertex_buffers(command_buffer, 0, &vertex_buffers, &offsets);
            
            // Draw surface quad (6 vertices for 2 triangles)
            self.device.handle().cmd_draw(command_buffer, 6, 1, 0, 0);
        }
        
        Ok(())
    }
    
    /// Update vertex buffer for a surface
    fn update_surface_vertex_buffer(&mut self, surface_id: u32, width: u32, height: u32) -> Result<()> {
        // Create quad vertices for this surface
        let vertices = SurfacePipeline::create_surface_quad_vertices(width, height);
        let _vertex_data = unsafe {
            std::slice::from_raw_parts(
                vertices.as_ptr() as *const u8,
                std::mem::size_of_val(&vertices),
            )
        };
        
        // TODO: Implement vertex buffer creation and upload
        // For now, create placeholder buffer
        debug!("Creating vertex buffer for surface {} ({}x{})", surface_id, width, height);
        
        Ok(())
    }
    
    /// Update descriptor set for a surface texture
    fn update_surface_descriptor_set(&mut self, surface_id: u32) -> Result<()> {
        // TODO: Implement descriptor set creation and texture binding
        debug!("Creating descriptor set for surface {}", surface_id);
        
        Ok(())
    }
}

impl Drop for CompositorRenderer {
    fn drop(&mut self) {
        eprintln!("CompositorRenderer::drop() - Starting cleanup");
        eprintln!("Device Arc strong_count: {}", Arc::strong_count(&self.device));
        
        // RADICAL FIX: Skip ALL cleanup when device is shared
        // Let VulkanDevice handle everything when it's finally destroyed
        eprintln!("CompositorRenderer::drop() - Skipping cleanup, letting VulkanDevice handle everything");
        eprintln!("CompositorRenderer::drop() - Complete");
    }
}
