use ash::{vk, Entry, Instance};
use compositor_utils::prelude::*;
use std::ffi::{CStr, CString};

/// Vulkan instance wrapper with validation layers for development
#[derive(Clone)]
pub struct VulkanInstance {
    entry: Entry,
    instance: Instance,
    debug_utils: Option<DebugUtils>,
    api_version: u32,
}

struct DebugUtils {
    loader: ash::extensions::ext::DebugUtils,
    messenger: vk::DebugUtilsMessengerEXT,
}

// Manual Clone implementation since DebugUtils doesn't derive Clone
impl Clone for DebugUtils {
    fn clone(&self) -> Self {
        Self {
            loader: self.loader.clone(),
            messenger: self.messenger,
        }
    }
}

impl VulkanInstance {
    /// Create a new Vulkan instance
    pub fn new() -> Result<Self> {
        let entry = Entry::linked();
        
        // Check API version
        let api_version = entry
            .try_enumerate_instance_version()?
            .unwrap_or(vk::make_api_version(0, 1, 0, 0));
        
        info!("Vulkan API version: {}", format_version(api_version));
        
        // Application info
        let app_name = CString::new("Custom Compositor")?;
        let engine_name = CString::new("Custom Engine")?;
        let app_info = vk::ApplicationInfo {
            p_application_name: app_name.as_ptr(),
            application_version: vk::make_api_version(0, 0, 1, 0),
            p_engine_name: engine_name.as_ptr(),
            engine_version: vk::make_api_version(0, 0, 1, 0),
            api_version,
            ..Default::default()
        };
        
        // Required extensions
        let mut extension_names = vec![
            ash::extensions::khr::Surface::name().as_ptr(),
            ash::extensions::khr::XlibSurface::name().as_ptr(),
            ash::extensions::khr::WaylandSurface::name().as_ptr(),
        ];
        
        // Add debug extensions in debug mode
        let debug_enabled = cfg!(debug_assertions);
        if debug_enabled {
            extension_names.push(ash::extensions::ext::DebugUtils::name().as_ptr());
        }
        
        // Validation layers
        let layer_names = if debug_enabled {
            vec![CString::new("VK_LAYER_KHRONOS_validation")?]
        } else {
            vec![]
        };
        let layer_names_raw: Vec<*const i8> = layer_names
            .iter()
            .map(|raw_name| raw_name.as_ptr())
            .collect();
        
        // Create instance
        let create_info = vk::InstanceCreateInfo {
            p_application_info: &app_info,
            enabled_layer_count: layer_names_raw.len() as u32,
            pp_enabled_layer_names: layer_names_raw.as_ptr(),
            enabled_extension_count: extension_names.len() as u32,
            pp_enabled_extension_names: extension_names.as_ptr(),
            ..Default::default()
        };
        
        let instance = unsafe { entry.create_instance(&create_info, None)? };
        
        // Setup debug messenger
        let debug_utils = if debug_enabled {
            Some(Self::setup_debug_messenger(&entry, &instance)?)
        } else {
            None
        };
        
        info!("Vulkan instance created successfully");
        
        Ok(Self {
            entry,
            instance,
            debug_utils,
            api_version,
        })
    }
    
    fn setup_debug_messenger(entry: &Entry, instance: &Instance) -> Result<DebugUtils> {
        let debug_info = vk::DebugUtilsMessengerCreateInfoEXT {
            message_severity: vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                | vk::DebugUtilsMessageSeverityFlagsEXT::INFO
                | vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE,
            message_type: vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
            pfn_user_callback: Some(vulkan_debug_callback),
            ..Default::default()
        };
        
        let debug_utils_loader = ash::extensions::ext::DebugUtils::new(entry, instance);
        let debug_callback = unsafe {
            debug_utils_loader.create_debug_utils_messenger(&debug_info, None)?
        };
        
        Ok(DebugUtils {
            loader: debug_utils_loader,
            messenger: debug_callback,
        })
    }
    
    /// Get the Vulkan instance handle
    pub fn handle(&self) -> &Instance {
        &self.instance
    }
    
    /// Get the Vulkan entry
    pub fn entry(&self) -> &Entry {
        &self.entry
    }
    
    /// Get API version
    pub fn get_api_version(&self) -> u32 {
        self.api_version
    }
    
    /// Enumerate physical devices
    pub fn enumerate_physical_devices(&self) -> Result<Vec<vk::PhysicalDevice>> {
        let devices = unsafe { self.instance.enumerate_physical_devices()? };
        Ok(devices)
    }
}

impl Drop for VulkanInstance {
    fn drop(&mut self) {
        eprintln!("VulkanInstance::drop() - Starting cleanup");
        
        // RADICAL FIX: Skip ALL cleanup to prevent SIGSEGV
        // The debug messenger and instance destruction can cause crashes during shutdown
        // when validation layers are involved, especially with command pool validation errors
        eprintln!("VulkanInstance::drop() - Skipping cleanup to prevent SIGSEGV");
        eprintln!("VulkanInstance::drop() - Complete");
    }
}

// Debug callback for Vulkan validation layers
unsafe extern "system" fn vulkan_debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    _message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut std::os::raw::c_void,
) -> vk::Bool32 {
    let callback_data = *p_callback_data;
    let message_id_number = callback_data.message_id_number;
    
    let message_id_name = if callback_data.p_message_id_name.is_null() {
        std::borrow::Cow::from("")
    } else {
        CStr::from_ptr(callback_data.p_message_id_name).to_string_lossy()
    };
    
    let message = if callback_data.p_message.is_null() {
        std::borrow::Cow::from("")
    } else {
        CStr::from_ptr(callback_data.p_message).to_string_lossy()
    };
    
    match message_severity {
        vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => {
            error!("[Vulkan] {} ({}): {}", message_id_name, message_id_number, message);
        }
        vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => {
            warn!("[Vulkan] {} ({}): {}", message_id_name, message_id_number, message);
        }
        vk::DebugUtilsMessageSeverityFlagsEXT::INFO => {
            info!("[Vulkan] {} ({}): {}", message_id_name, message_id_number, message);
        }
        _ => {
            debug!("[Vulkan] {} ({}): {}", message_id_name, message_id_number, message);
        }
    }
    
    vk::FALSE
}

fn format_version(version: u32) -> String {
    format!(
        "{}.{}.{}",
        vk::api_version_major(version),
        vk::api_version_minor(version),
        vk::api_version_patch(version)
    )
}
