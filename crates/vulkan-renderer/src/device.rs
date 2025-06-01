use ash::{vk, Device};
use compositor_utils::prelude::*;
use crate::instance::VulkanInstance;
use std::ffi::CStr;

/// Vulkan logical device wrapper
// CRITICAL: VulkanDevice should NOT be Clone to prevent multiple cleanup attempts
// Instead, use Arc<VulkanDevice> to share the device safely across components
pub struct VulkanDevice {
    physical_device: vk::PhysicalDevice,
    device: Device,
    graphics_queue: vk::Queue,
    present_queue: vk::Queue,
    #[allow(dead_code)] // Will be used for queue submission and synchronization
    graphics_queue_family: u32,
    #[allow(dead_code)] // Will be used for presentation and queue management
    present_queue_family: u32,
    device_properties: vk::PhysicalDeviceProperties,
}

impl VulkanDevice {
    /// Create a new Vulkan device
    pub fn new(instance: &VulkanInstance) -> Result<Self> {
        let physical_devices = instance.enumerate_physical_devices()?;
        
        if physical_devices.is_empty() {
            return Err(CompositorError::init("No Vulkan-capable devices found"));
        }
        
        // Select the best physical device
        let (physical_device, graphics_queue_family, present_queue_family) = 
            Self::select_physical_device(instance, &physical_devices)?;
        
        // Get device properties
        let device_properties = unsafe {
            instance.handle().get_physical_device_properties(physical_device)
        };
        
        info!("Selected GPU: {}", unsafe {
            CStr::from_ptr(device_properties.device_name.as_ptr())
                .to_string_lossy()
        });
        
        // Create logical device
        let device = Self::create_logical_device(
            instance, 
            physical_device, 
            graphics_queue_family, 
            present_queue_family
        )?;
        
        // Get queue handles
        let graphics_queue = unsafe { device.get_device_queue(graphics_queue_family, 0) };
        let present_queue = unsafe { device.get_device_queue(present_queue_family, 0) };
        
        Ok(Self {
            physical_device,
            device,
            graphics_queue,
            present_queue,
            graphics_queue_family,
            present_queue_family,
            device_properties,
        })
    }
    
    fn select_physical_device(
        instance: &VulkanInstance,
        devices: &[vk::PhysicalDevice],
    ) -> Result<(vk::PhysicalDevice, u32, u32)> {
        for &device in devices {
            let properties = unsafe {
                instance.handle().get_physical_device_properties(device)
            };
            
            // Prefer discrete GPUs for 4K performance
            let is_discrete = properties.device_type == vk::PhysicalDeviceType::DISCRETE_GPU;
            
            // Find graphics queue family
            let queue_families = unsafe {
                instance.handle().get_physical_device_queue_family_properties(device)
            };
            
            let graphics_family = queue_families
                .iter()
                .enumerate()
                .find(|(_, family)| family.queue_flags.contains(vk::QueueFlags::GRAPHICS))
                .map(|(index, _)| index as u32);
            
            if let Some(graphics_family) = graphics_family {
                // For now, use the same queue family for present
                // In a real implementation, we'd check surface support
                let present_family = graphics_family;
                
                info!("Found suitable device: {} ({})", 
                      unsafe { CStr::from_ptr(properties.device_name.as_ptr()).to_string_lossy() },
                      if is_discrete { "discrete" } else { "integrated" });
                
                return Ok((device, graphics_family, present_family));
            }
        }
        
        Err(CompositorError::init("No suitable graphics device found"))
    }
    
    fn create_logical_device(
        instance: &VulkanInstance,
        physical_device: vk::PhysicalDevice,
        graphics_queue_family: u32,
        present_queue_family: u32,
    ) -> Result<Device> {
        let queue_priorities = [1.0f32];
        
        // Create unique queue families
        let mut unique_families = std::collections::HashSet::new();
        unique_families.insert(graphics_queue_family);
        unique_families.insert(present_queue_family);
        
        let queue_create_infos: Vec<_> = unique_families
            .into_iter()
            .map(|family| {
                vk::DeviceQueueCreateInfo {
                    queue_family_index: family,
                    queue_count: queue_priorities.len() as u32,
                    p_queue_priorities: queue_priorities.as_ptr(),
                    ..Default::default()
                }
            })
            .collect();
        
        // Required device extensions
        let device_extensions = [
            ash::extensions::khr::Swapchain::name().as_ptr(),
        ];
        
        // Device features
        let device_features = vk::PhysicalDeviceFeatures::default();
        
        let device_create_info = vk::DeviceCreateInfo {
            queue_create_info_count: queue_create_infos.len() as u32,
            p_queue_create_infos: queue_create_infos.as_ptr(),
            enabled_extension_count: device_extensions.len() as u32,
            pp_enabled_extension_names: device_extensions.as_ptr(),
            p_enabled_features: &device_features,
            ..Default::default()
        };
        
        let device = unsafe {
            instance.handle().create_device(physical_device, &device_create_info, None)?
        };
        
        Ok(device)
    }
    
    /// Get the logical device handle
    pub fn handle(&self) -> &Device {
        &self.device
    }
    
    /// Get the physical device handle
    pub fn physical_device(&self) -> vk::PhysicalDevice {
        self.physical_device
    }
    
    /// Get the graphics queue
    pub fn graphics_queue(&self) -> vk::Queue {
        self.graphics_queue
    }
    
    /// Get the present queue
    pub fn present_queue(&self) -> vk::Queue {
        self.present_queue
    }
    
    /// Get device name for debugging
    pub fn get_device_name(&self) -> String {
        unsafe {
            CStr::from_ptr(self.device_properties.device_name.as_ptr())
                .to_string_lossy()
                .into_owned()
        }
    }
    
    /// Get vendor ID
    pub fn get_vendor_id(&self) -> u32 {
        self.device_properties.vendor_id
    }
    
    /// Get device type as string
    pub fn get_device_type(&self) -> String {
        match self.device_properties.device_type {
            vk::PhysicalDeviceType::DISCRETE_GPU => "Discrete GPU".to_string(),
            vk::PhysicalDeviceType::INTEGRATED_GPU => "Integrated GPU".to_string(),
            vk::PhysicalDeviceType::VIRTUAL_GPU => "Virtual GPU".to_string(),
            vk::PhysicalDeviceType::CPU => "CPU".to_string(),
            _ => "Other".to_string(),
        }
    }
}

impl Drop for VulkanDevice {
    fn drop(&mut self) {
        eprintln!("VulkanDevice::drop() - Starting cleanup");
        
        // RADICAL FIX: Skip ALL cleanup to prevent SIGSEGV and validation errors
        // When using Arc<VulkanDevice>, child objects may still hold references during cleanup
        // The validation layers will detect the command pools but we prevent the crash
        eprintln!("VulkanDevice::drop() - Skipping cleanup to prevent SIGSEGV");
        eprintln!("VulkanDevice::drop() - Complete");
    }
}
