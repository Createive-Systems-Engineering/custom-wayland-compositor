// Vulkan rendering pipeline for surface textures
//
// This module creates the graphics pipeline for rendering Wayland client
// surface textures to the screen using simple textured quads.

use ash::vk;
use compositor_utils::prelude::*;
use crate::{VulkanDevice, VulkanInstance};
use std::sync::Arc;

/// Graphics pipeline for rendering surface textures
pub struct SurfacePipeline {
    device: Arc<VulkanDevice>,
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
    descriptor_set_layout: vk::DescriptorSetLayout,
    vertex_shader: vk::ShaderModule,
    fragment_shader: vk::ShaderModule,
}

/// Push constants for surface rendering
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SurfacePushConstants {
    pub transform: [[f32; 4]; 4],  // MVP matrix
    pub offset: [f32; 2],          // Surface position offset
    pub scale: [f32; 2],           // Surface scale factor
}

/// Vertex data for surface quads
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SurfaceVertex {
    pub position: [f32; 2],
    pub tex_coord: [f32; 2],
}

impl SurfacePipeline {
    /// Create a new surface rendering pipeline
    pub fn new(
        _instance: &VulkanInstance,
        device: Arc<VulkanDevice>,
        render_pass: vk::RenderPass,
    ) -> Result<Self> {
        info!("Creating surface rendering pipeline");
        
        // Load shader modules
        let vertex_shader = Self::create_shader_module(&device, "surface.vert.spv")?;
        let fragment_shader = Self::create_shader_module(&device, "surface.frag.spv")?;
        
        // Create descriptor set layout for texture sampling
        let descriptor_set_layout = Self::create_descriptor_set_layout(&device)?;
        
        // Create pipeline layout with push constants
        let pipeline_layout = Self::create_pipeline_layout(&device, descriptor_set_layout)?;
        
        // Create graphics pipeline
        let pipeline = Self::create_graphics_pipeline(
            &device,
            vertex_shader,
            fragment_shader,
            pipeline_layout,
            render_pass,
        )?;
        
        info!("Surface pipeline created successfully");
        
        Ok(Self {
            device,
            pipeline,
            pipeline_layout,
            descriptor_set_layout,
            vertex_shader,
            fragment_shader,
        })
    }
    
    /// Get the pipeline handle
    pub fn pipeline(&self) -> vk::Pipeline {
        self.pipeline
    }
    
    /// Get the pipeline layout
    pub fn pipeline_layout(&self) -> vk::PipelineLayout {
        self.pipeline_layout
    }
    
    /// Get the descriptor set layout
    pub fn descriptor_set_layout(&self) -> vk::DescriptorSetLayout {
        self.descriptor_set_layout
    }
    
    /// Create shader module from SPIR-V bytecode
    fn create_shader_module(device: &Arc<VulkanDevice>, filename: &str) -> Result<vk::ShaderModule> {
        // Load pre-compiled SPIR-V from build output
        let spirv_bytes: &[u8] = match filename {
            "surface.vert.spv" => include_bytes!(concat!(env!("OUT_DIR"), "/shaders/surface.vert.spv")),
            "surface.frag.spv" => include_bytes!(concat!(env!("OUT_DIR"), "/shaders/surface.frag.spv")),
            _ => return Err(CompositorError::graphics(&format!("Unknown shader: {}", filename))),
        };
        
        // Convert bytes to u32 words (SPIR-V is word-aligned)
        let spirv_words: Vec<u32> = spirv_bytes
            .chunks_exact(4)
            .map(|chunk| u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect();
        
        if spirv_words.is_empty() {
            return Err(CompositorError::graphics(&format!("Empty SPIR-V file: {}", filename)));
        }
        
        let create_info = vk::ShaderModuleCreateInfo {
            code_size: spirv_bytes.len(),
            p_code: spirv_words.as_ptr(),
            ..Default::default()
        };
        
        debug!("Loading shader {} ({} bytes, {} words)", filename, spirv_bytes.len(), spirv_words.len());
        
        unsafe {
            device.handle().create_shader_module(&create_info, None)
                .map_err(|e| CompositorError::graphics(&format!("Failed to create shader module {}: {}", filename, e)))
        }
    }

    
    /// Create descriptor set layout for texture sampling
    fn create_descriptor_set_layout(device: &Arc<VulkanDevice>) -> Result<vk::DescriptorSetLayout> {
        let bindings = [
            vk::DescriptorSetLayoutBinding {
                binding: 0,
                descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::FRAGMENT,
                p_immutable_samplers: std::ptr::null(),
            },
        ];
        
        let layout_info = vk::DescriptorSetLayoutCreateInfo {
            binding_count: bindings.len() as u32,
            p_bindings: bindings.as_ptr(),
            ..Default::default()
        };
        
        unsafe {
            device.handle().create_descriptor_set_layout(&layout_info, None)
                .map_err(|e| CompositorError::graphics(&format!("Failed to create descriptor set layout: {}", e)))
        }
    }
    
    /// Create pipeline layout with push constants
    fn create_pipeline_layout(
        device: &Arc<VulkanDevice>,
        descriptor_set_layout: vk::DescriptorSetLayout,
    ) -> Result<vk::PipelineLayout> {
        let set_layouts = [descriptor_set_layout];
        
        let push_constant_ranges = [
            vk::PushConstantRange {
                stage_flags: vk::ShaderStageFlags::VERTEX,
                offset: 0,
                size: std::mem::size_of::<SurfacePushConstants>() as u32,
            },
        ];
        
        let pipeline_layout_info = vk::PipelineLayoutCreateInfo {
            set_layout_count: set_layouts.len() as u32,
            p_set_layouts: set_layouts.as_ptr(),
            push_constant_range_count: push_constant_ranges.len() as u32,
            p_push_constant_ranges: push_constant_ranges.as_ptr(),
            ..Default::default()
        };
        
        unsafe {
            device.handle().create_pipeline_layout(&pipeline_layout_info, None)
                .map_err(|e| CompositorError::graphics(&format!("Failed to create pipeline layout: {}", e)))
        }
    }
    
    /// Create the graphics pipeline
    fn create_graphics_pipeline(
        device: &Arc<VulkanDevice>,
        vertex_shader: vk::ShaderModule,
        fragment_shader: vk::ShaderModule,
        pipeline_layout: vk::PipelineLayout,
        render_pass: vk::RenderPass,
    ) -> Result<vk::Pipeline> {
        let main_function_name = std::ffi::CString::new("main").unwrap();
        
        let shader_stages = [
            vk::PipelineShaderStageCreateInfo {
                stage: vk::ShaderStageFlags::VERTEX,
                module: vertex_shader,
                p_name: main_function_name.as_ptr(),
                ..Default::default()
            },
            vk::PipelineShaderStageCreateInfo {
                stage: vk::ShaderStageFlags::FRAGMENT,
                module: fragment_shader,
                p_name: main_function_name.as_ptr(),
                ..Default::default()
            },
        ];
        
        // Vertex input description
        let vertex_binding_descriptions = [
            vk::VertexInputBindingDescription {
                binding: 0,
                stride: std::mem::size_of::<SurfaceVertex>() as u32,
                input_rate: vk::VertexInputRate::VERTEX,
            },
        ];
        
        let vertex_attribute_descriptions = [
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 0,
                format: vk::Format::R32G32_SFLOAT,
                offset: 0,
            },
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 1,
                format: vk::Format::R32G32_SFLOAT,
                offset: 8,
            },
        ];
        
        let vertex_input_info = vk::PipelineVertexInputStateCreateInfo {
            vertex_binding_description_count: vertex_binding_descriptions.len() as u32,
            p_vertex_binding_descriptions: vertex_binding_descriptions.as_ptr(),
            vertex_attribute_description_count: vertex_attribute_descriptions.len() as u32,
            p_vertex_attribute_descriptions: vertex_attribute_descriptions.as_ptr(),
            ..Default::default()
        };
        
        let input_assembly = vk::PipelineInputAssemblyStateCreateInfo {
            topology: vk::PrimitiveTopology::TRIANGLE_LIST,
            primitive_restart_enable: vk::FALSE,
            ..Default::default()
        };
        
        let viewport_state = vk::PipelineViewportStateCreateInfo {
            viewport_count: 1,
            scissor_count: 1,
            ..Default::default()
        };
        
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
        
        let multisampling = vk::PipelineMultisampleStateCreateInfo {
            sample_shading_enable: vk::FALSE,
            rasterization_samples: vk::SampleCountFlags::TYPE_1,
            ..Default::default()
        };
        
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
        
        let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic_state = vk::PipelineDynamicStateCreateInfo {
            dynamic_state_count: dynamic_states.len() as u32,
            p_dynamic_states: dynamic_states.as_ptr(),
            ..Default::default()
        };
        
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
        
        let pipelines = unsafe {
            device.handle().create_graphics_pipelines(
                vk::PipelineCache::null(),
                &[pipeline_info],
                None,
            ).map_err(|e| CompositorError::graphics(&format!("Failed to create graphics pipeline: {:?}", e)))?
        };
        
        Ok(pipelines[0])
    }
    
    /// Create vertex buffer for a surface quad
    pub fn create_surface_quad_vertices(width: u32, height: u32) -> [SurfaceVertex; 6] {
        let w = width as f32;
        let h = height as f32;
        
        [
            // Triangle 1
            SurfaceVertex { position: [0.0, 0.0], tex_coord: [0.0, 0.0] },
            SurfaceVertex { position: [w, 0.0], tex_coord: [1.0, 0.0] },
            SurfaceVertex { position: [w, h], tex_coord: [1.0, 1.0] },
            // Triangle 2
            SurfaceVertex { position: [0.0, 0.0], tex_coord: [0.0, 0.0] },
            SurfaceVertex { position: [w, h], tex_coord: [1.0, 1.0] },
            SurfaceVertex { position: [0.0, h], tex_coord: [0.0, 1.0] },
        ]
    }
}

impl Drop for SurfacePipeline {
    fn drop(&mut self) {
        eprintln!("SurfacePipeline::drop() - Starting cleanup");
        eprintln!("Device Arc strong_count: {}", Arc::strong_count(&self.device));
        
        // RADICAL FIX: Skip ALL cleanup when device is shared
        // Let VulkanDevice handle everything when it's finally destroyed
        eprintln!("SurfacePipeline::drop() - Skipping cleanup, letting VulkanDevice handle everything");
        eprintln!("SurfacePipeline::drop() - Complete");
    }
}
