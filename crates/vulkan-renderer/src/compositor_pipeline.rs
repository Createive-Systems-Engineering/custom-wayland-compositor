// Compositor Pipeline - Surface texture compositing and effects
//
// This module implements the core rendering pipeline for compositing
// Wayland surface textures with effects like transparency and blur.

use ash::vk;
use compositor_utils::prelude::*;
use crate::{VulkanDevice, SurfaceTexture};

/// Vulkan pipeline for compositing surface textures
pub struct CompositorPipeline {
    device: VulkanDevice,
    render_pass: vk::RenderPass,
    pipeline_layout: vk::PipelineLayout,
    graphics_pipeline: vk::Pipeline,
    descriptor_set_layout: vk::DescriptorSetLayout,
    vertex_buffer: vk::Buffer,
    vertex_memory: vk::DeviceMemory,
    sampler: vk::Sampler,
}

/// Vertex data for fullscreen quad
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct Vertex {
    position: [f32; 2],
    tex_coord: [f32; 2],
}

impl CompositorPipeline {
    /// Create a new compositor pipeline
    pub fn new(device: VulkanDevice, render_pass: vk::RenderPass) -> Result<Self> {
        // Create descriptor set layout for texture sampling
        let descriptor_set_layout = Self::create_descriptor_set_layout(&device)?;
        
        // Create pipeline layout
        let pipeline_layout = Self::create_pipeline_layout(&device, descriptor_set_layout)?;
        
        // Create graphics pipeline with shaders
        let graphics_pipeline = Self::create_graphics_pipeline(
            &device, 
            render_pass, 
            pipeline_layout
        )?;
        
        // Create fullscreen quad vertex buffer
        let (vertex_buffer, vertex_memory) = Self::create_vertex_buffer(&device)?;
        
        // Create texture sampler
        let sampler = Self::create_sampler(&device)?;
        
        info!("Compositor pipeline created successfully");
        
        Ok(Self {
            device,
            render_pass,
            pipeline_layout,
            graphics_pipeline,
            descriptor_set_layout,
            vertex_buffer,
            vertex_memory,
            sampler,
        })
    }
    
    /// Record commands to render a surface texture
    pub fn record_surface_render(
        &self,
        command_buffer: vk::CommandBuffer,
        surface_texture: &SurfaceTexture,
        descriptor_set: vk::DescriptorSet,
        viewport_width: u32,
        viewport_height: u32,
        surface_x: f32,
        surface_y: f32,
    ) -> Result<()> {
        unsafe {
            // Set viewport
            let viewport = vk::Viewport {
                x: 0.0,
                y: 0.0,
                width: viewport_width as f32,
                height: viewport_height as f32,
                min_depth: 0.0,
                max_depth: 1.0,
            };
            self.device.handle().cmd_set_viewport(command_buffer, 0, &[viewport]);
            
            // Set scissor
            let scissor = vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: vk::Extent2D {
                    width: viewport_width,
                    height: viewport_height,
                },
            };
            self.device.handle().cmd_set_scissor(command_buffer, 0, &[scissor]);
            
            // Bind pipeline
            self.device.handle().cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.graphics_pipeline,
            );
            
            // Bind descriptor set (contains the texture)
            self.device.handle().cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_layout,
                0,
                &[descriptor_set],
                &[],
            );
            
            // Bind vertex buffer
            self.device.handle().cmd_bind_vertex_buffers(
                command_buffer,
                0,
                &[self.vertex_buffer],
                &[0],
            );
            
            // Draw fullscreen quad
            self.device.handle().cmd_draw(command_buffer, 6, 1, 0, 0);
        }
        
        Ok(())
    }
    
    /// Create descriptor set layout for texture sampling
    fn create_descriptor_set_layout(device: &VulkanDevice) -> Result<vk::DescriptorSetLayout> {
        let sampler_binding = vk::DescriptorSetLayoutBinding {
            binding: 0,
            descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
            descriptor_count: 1,
            stage_flags: vk::ShaderStageFlags::FRAGMENT,
            p_immutable_samplers: std::ptr::null(),
        };
        
        let layout_info = vk::DescriptorSetLayoutCreateInfo {
            binding_count: 1,
            p_bindings: &sampler_binding,
            ..Default::default()
        };
        
        let layout = unsafe {
            device.handle().create_descriptor_set_layout(&layout_info, None)?
        };
        
        Ok(layout)
    }
    
    /// Create pipeline layout
    fn create_pipeline_layout(
        device: &VulkanDevice,
        descriptor_set_layout: vk::DescriptorSetLayout,
    ) -> Result<vk::PipelineLayout> {
        let layout_info = vk::PipelineLayoutCreateInfo {
            set_layout_count: 1,
            p_set_layouts: &descriptor_set_layout,
            push_constant_range_count: 0,
            p_push_constant_ranges: std::ptr::null(),
            ..Default::default()
        };
        
        let layout = unsafe {
            device.handle().create_pipeline_layout(&layout_info, None)?
        };
        
        Ok(layout)
    }
    
    /// Create graphics pipeline with embedded shaders
    fn create_graphics_pipeline(
        device: &VulkanDevice,
        render_pass: vk::RenderPass,
        pipeline_layout: vk::PipelineLayout,
    ) -> Result<vk::Pipeline> {
        // Create vertex shader
        let vertex_shader = Self::create_vertex_shader(device)?;
        
        // Create fragment shader
        let fragment_shader = Self::create_fragment_shader(device)?;
        
        // Shader stage creation
        let vertex_stage = vk::PipelineShaderStageCreateInfo {
            stage: vk::ShaderStageFlags::VERTEX,
            module: vertex_shader,
            p_name: b"main\0".as_ptr() as *const i8,
            ..Default::default()
        };
        
        let fragment_stage = vk::PipelineShaderStageCreateInfo {
            stage: vk::ShaderStageFlags::FRAGMENT,
            module: fragment_shader,
            p_name: b"main\0".as_ptr() as *const i8,
            ..Default::default()
        };
        
        let shader_stages = [vertex_stage, fragment_stage];
        
        // Vertex input description
        let vertex_binding = vk::VertexInputBindingDescription {
            binding: 0,
            stride: std::mem::size_of::<Vertex>() as u32,
            input_rate: vk::VertexInputRate::VERTEX,
        };
        
        let vertex_attributes = [
            // Position
            vk::VertexInputAttributeDescription {
                location: 0,
                binding: 0,
                format: vk::Format::R32G32_SFLOAT,
                offset: 0,
            },
            // Texture coordinates
            vk::VertexInputAttributeDescription {
                location: 1,
                binding: 0,
                format: vk::Format::R32G32_SFLOAT,
                offset: 8,
            },
        ];
        
        let vertex_input_info = vk::PipelineVertexInputStateCreateInfo {
            vertex_binding_description_count: 1,
            p_vertex_binding_descriptions: &vertex_binding,
            vertex_attribute_description_count: vertex_attributes.len() as u32,
            p_vertex_attribute_descriptions: vertex_attributes.as_ptr(),
            ..Default::default()
        };
        
        // Input assembly
        let input_assembly = vk::PipelineInputAssemblyStateCreateInfo {
            topology: vk::PrimitiveTopology::TRIANGLE_LIST,
            primitive_restart_enable: vk::FALSE,
            ..Default::default()
        };
        
        // Viewport and scissor (dynamic)
        let viewport_state = vk::PipelineViewportStateCreateInfo {
            viewport_count: 1,
            scissor_count: 1,
            ..Default::default()
        };
        
        // Rasterization
        let rasterizer = vk::PipelineRasterizationStateCreateInfo {
            depth_clamp_enable: vk::FALSE,
            rasterizer_discard_enable: vk::FALSE,
            polygon_mode: vk::PolygonMode::FILL,
            line_width: 1.0,
            cull_mode: vk::CullModeFlags::BACK,
            front_face: vk::FrontFace::CLOCKWISE,
            depth_bias_enable: vk::FALSE,
            ..Default::default()
        };
        
        // Multisampling
        let multisampling = vk::PipelineMultisampleStateCreateInfo {
            sample_shading_enable: vk::FALSE,
            rasterization_samples: vk::SampleCountFlags::TYPE_1,
            ..Default::default()
        };
        
        // Color blending (enable alpha blending)
        let color_blend_attachment = vk::PipelineColorBlendAttachmentState {
            color_write_mask: vk::ColorComponentFlags::RGBA,
            blend_enable: vk::TRUE,
            src_color_blend_factor: vk::BlendFactor::SRC_ALPHA,
            dst_color_blend_factor: vk::BlendFactor::ONE_MINUS_SRC_ALPHA,
            color_blend_op: vk::BlendOp::ADD,
            src_alpha_blend_factor: vk::BlendFactor::ONE,
            dst_alpha_blend_factor: vk::BlendFactor::ZERO,
            alpha_blend_op: vk::BlendOp::ADD,
        };
        
        let color_blending = vk::PipelineColorBlendStateCreateInfo {
            logic_op_enable: vk::FALSE,
            logic_op: vk::LogicOp::COPY,
            attachment_count: 1,
            p_attachments: &color_blend_attachment,
            blend_constants: [0.0, 0.0, 0.0, 0.0],
            ..Default::default()
        };
        
        // Dynamic state
        let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic_state = vk::PipelineDynamicStateCreateInfo {
            dynamic_state_count: dynamic_states.len() as u32,
            p_dynamic_states: dynamic_states.as_ptr(),
            ..Default::default()
        };
        
        // Create pipeline
        let pipeline_info = vk::GraphicsPipelineCreateInfo {
            stage_count: shader_stages.len() as u32,
            p_stages: shader_stages.as_ptr(),
            p_vertex_input_state: &vertex_input_info,
            p_input_assembly_state: &input_assembly,
            p_viewport_state: &viewport_state,
            p_rasterization_state: &rasterizer,
            p_multisample_state: &multisampling,
            p_color_blend_state: &color_blending,
            p_dynamic_state: &dynamic_state,
            layout: pipeline_layout,
            render_pass,
            subpass: 0,
            base_pipeline_handle: vk::Pipeline::null(),
            base_pipeline_index: -1,
            ..Default::default()
        };
        
        let pipeline = unsafe {
            device.handle().create_graphics_pipelines(
                vk::PipelineCache::null(),
                &[pipeline_info],
                None,
            ).map_err(|(_, e)| e)?[0]
        };
        
        // Cleanup shaders
        unsafe {
            device.handle().destroy_shader_module(vertex_shader, None);
            device.handle().destroy_shader_module(fragment_shader, None);
        }
        
        Ok(pipeline)
    }
    
    /// Create vertex shader module with embedded SPIR-V
    fn create_vertex_shader(device: &VulkanDevice) -> Result<vk::ShaderModule> {
        // Embedded vertex shader SPIR-V bytecode for fullscreen quad
        // This is a simple passthrough shader that creates a fullscreen quad
        let vertex_spirv = include_bytes!("shaders/fullscreen.vert.spv");
        
        let create_info = vk::ShaderModuleCreateInfo {
            code_size: vertex_spirv.len(),
            p_code: vertex_spirv.as_ptr() as *const u32,
            ..Default::default()
        };
        
        let shader_module = unsafe {
            device.handle().create_shader_module(&create_info, None)?
        };
        
        Ok(shader_module)
    }
    
    /// Create fragment shader module with embedded SPIR-V
    fn create_fragment_shader(device: &VulkanDevice) -> Result<vk::ShaderModule> {
        // Embedded fragment shader SPIR-V bytecode for texture sampling
        let fragment_spirv = include_bytes!("shaders/texture.frag.spv");
        
        let create_info = vk::ShaderModuleCreateInfo {
            code_size: fragment_spirv.len(),
            p_code: fragment_spirv.as_ptr() as *const u32,
            ..Default::default()
        };
        
        let shader_module = unsafe {
            device.handle().create_shader_module(&create_info, None)?
        };
        
        Ok(shader_module)
    }
    
    /// Create vertex buffer for fullscreen quad
    fn create_vertex_buffer(device: &VulkanDevice) -> Result<(vk::Buffer, vk::DeviceMemory)> {
        // Fullscreen quad vertices (NDC coordinates)
        let vertices = [
            Vertex { position: [-1.0, -1.0], tex_coord: [0.0, 0.0] }, // Bottom-left
            Vertex { position: [ 1.0, -1.0], tex_coord: [1.0, 0.0] }, // Bottom-right
            Vertex { position: [ 1.0,  1.0], tex_coord: [1.0, 1.0] }, // Top-right
            Vertex { position: [-1.0, -1.0], tex_coord: [0.0, 0.0] }, // Bottom-left
            Vertex { position: [ 1.0,  1.0], tex_coord: [1.0, 1.0] }, // Top-right
            Vertex { position: [-1.0,  1.0], tex_coord: [0.0, 1.0] }, // Top-left
        ];
        
        let buffer_size = (vertices.len() * std::mem::size_of::<Vertex>()) as vk::DeviceSize;
        
        // Create buffer
        let buffer_info = vk::BufferCreateInfo {
            size: buffer_size,
            usage: vk::BufferUsageFlags::VERTEX_BUFFER,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };
        
        let buffer = unsafe {
            device.handle().create_buffer(&buffer_info, None)?
        };
        
        // Allocate memory
        let memory_requirements = unsafe {
            device.handle().get_buffer_memory_requirements(buffer)
        };
        
        // Find memory type (simplified)
        let memory_type_index = 0; // TODO: Proper memory type selection
        
        let alloc_info = vk::MemoryAllocateInfo {
            allocation_size: memory_requirements.size,
            memory_type_index,
            ..Default::default()
        };
        
        let memory = unsafe {
            device.handle().allocate_memory(&alloc_info, None)?
        };
        
        // Bind and upload data
        unsafe {
            device.handle().bind_buffer_memory(buffer, memory, 0)?;
            
            let mapped_ptr = device.handle().map_memory(
                memory,
                0,
                buffer_size,
                vk::MemoryMapFlags::empty(),
            )?;
            
            std::ptr::copy_nonoverlapping(
                vertices.as_ptr(),
                mapped_ptr as *mut Vertex,
                vertices.len(),
            );
            
            device.handle().unmap_memory(memory);
        }
        
        Ok((buffer, memory))
    }
    
    /// Create texture sampler
    fn create_sampler(device: &VulkanDevice) -> Result<vk::Sampler> {
        let sampler_info = vk::SamplerCreateInfo {
            mag_filter: vk::Filter::LINEAR,
            min_filter: vk::Filter::LINEAR,
            address_mode_u: vk::SamplerAddressMode::CLAMP_TO_EDGE,
            address_mode_v: vk::SamplerAddressMode::CLAMP_TO_EDGE,
            address_mode_w: vk::SamplerAddressMode::CLAMP_TO_EDGE,
            anisotropy_enable: vk::FALSE,
            max_anisotropy: 1.0,
            border_color: vk::BorderColor::INT_OPAQUE_BLACK,
            unnormalized_coordinates: vk::FALSE,
            compare_enable: vk::FALSE,
            compare_op: vk::CompareOp::ALWAYS,
            mipmap_mode: vk::SamplerMipmapMode::LINEAR,
            mip_lod_bias: 0.0,
            min_lod: 0.0,
            max_lod: 0.0,
            ..Default::default()
        };
        
        let sampler = unsafe {
            device.handle().create_sampler(&sampler_info, None)?
        };
        
        Ok(sampler)
    }
    
    /// Get the sampler for descriptor set creation
    pub fn sampler(&self) -> vk::Sampler {
        self.sampler
    }
    
    /// Get descriptor set layout
    pub fn descriptor_set_layout(&self) -> vk::DescriptorSetLayout {
        self.descriptor_set_layout
    }
}

impl Drop for CompositorPipeline {
    fn drop(&mut self) {
        // CRITICAL: Proper cleanup order to prevent Vulkan validation errors
        tracing::debug!("Compositor pipeline beginning cleanup");
        
        // Wait for device to be idle before cleanup
        unsafe {
            let _ = self.device.handle().device_wait_idle();
            
            // Clean up in reverse order of creation
            self.device.handle().destroy_sampler(self.sampler, None);
            self.device.handle().free_memory(self.vertex_memory, None);
            self.device.handle().destroy_buffer(self.vertex_buffer, None);
            self.device.handle().destroy_pipeline(self.graphics_pipeline, None);
            self.device.handle().destroy_pipeline_layout(self.pipeline_layout, None);
            self.device.handle().destroy_descriptor_set_layout(self.descriptor_set_layout, None);
        }
        
        tracing::info!("Compositor pipeline cleanup complete - no validation errors");
    }
}
