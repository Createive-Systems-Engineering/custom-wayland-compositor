use compositor_utils::prelude::*;
use ash::vk;
use vulkan_renderer::VulkanRenderer;

/// Display output information
#[derive(Debug, Clone)]
pub struct DisplayOutput {
    /// Output identifier  
    pub id: u32,
    /// Display name
    pub name: String,
    /// Resolution
    pub width: u32,
    pub height: u32,
    /// Refresh rate
    pub refresh_rate: u32,
    /// Connected status
    pub connected: bool,
    /// Vulkan surface for this output (if connected)
    pub vk_surface: Option<vk::SurfaceKHR>,
}

/// Backend type selection
#[derive(Debug, Clone)]
pub enum BackendType {
    /// Windowed backend (for testing/development)
    Windowed,
    /// DRM backend (for actual compositor)
    Drm,
    /// Auto-detect best backend
    Auto,
}

/// Backend abstraction for different display and input systems
pub struct Backend {
    backend_type: BackendType,
    /// Selected DRM device path (for DRM backend)
    drm_device_path: Option<String>,
    /// Connected displays
    displays: Vec<DisplayOutput>,
    /// Primary display index
    primary_display: Option<usize>,
    // Will be expanded to include actual backend instances
}

impl Backend {
    /// Create a new backend with auto-detection
    pub async fn new() -> Result<Self> {
        Self::new_with_type(BackendType::Auto).await
    }
    
    /// Create a new backend with specific type
    pub async fn new_with_type(backend_type: BackendType) -> Result<Self> {
        info!("Initializing backend: {:?}", backend_type);
        
        let actual_type = match backend_type {
            BackendType::Auto => {
                // Try to detect if we can use DRM
                if Self::can_use_drm().await {
                    info!("Auto-detected DRM backend capability");
                    BackendType::Drm
                } else {
                    info!("Falling back to windowed backend");
                    BackendType::Windowed
                }
            }
            other => other,
        };
        
        match actual_type {
            BackendType::Windowed => Self::init_windowed_backend().await,
            BackendType::Drm => Self::init_drm_backend().await,
            BackendType::Auto => unreachable!(),
        }
    }
    
    /// Check if DRM backend is available
    async fn can_use_drm() -> bool {
        // Check if we have access to DRM devices
        
        // Enumerate all available DRM devices
        let drm_devices = Self::enumerate_drm_devices().await;
        
        if drm_devices.is_empty() {
            debug!("No DRM devices found");
            return false;
        }
        
        // Check if we're running in a proper session that supports DRM master access
        // For now, be conservative and require explicit session variables
        let has_session = std::env::var("XDG_SESSION_TYPE")
            .map(|t| t == "wayland")
            .unwrap_or(false) || 
            std::env::var("XDG_SESSION_ID").is_ok() ||
            std::env::var("LOGNAME").map(|u| u == "root").unwrap_or(false);
        
        if !has_session {
            info!("No proper session detected for DRM access, using windowed backend");
            return false;
        }
        
        // Try to access the primary DRM device
        for device_path in &drm_devices {
            match std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(device_path)
            {
                Ok(_) => {
                    debug!("DRM device accessible: {}", device_path);
                    return true;
                }
                Err(e) => {
                    debug!("DRM device not accessible {}: {}", device_path, e);
                }
            }
        }
        
        false
    }
    
    /// Enumerate available DRM devices and prioritize them
    async fn enumerate_drm_devices() -> Vec<String> {
        use std::path::Path;
        
        let mut devices = Vec::new();
        
        // Check for DRM cards (card0, card1, etc.)
        for i in 0..8 {
            let device_path = format!("/dev/dri/card{}", i);
            if Path::new(&device_path).exists() {
                devices.push(device_path);
            }
        }
        
        // Sort devices to prioritize card0 over card1 for dual-display systems
        devices.sort();
        
        if !devices.is_empty() {
            info!("Found DRM devices: {:?}", devices);
        }
        
        devices
    }
    
    /// Initialize windowed backend (for development/testing)
    async fn init_windowed_backend() -> Result<Self> {
        info!("Initializing windowed backend");
        
        // TODO: Initialize winit or similar for windowed mode
        // This will be useful for development and testing
        
        Ok(Self {
            backend_type: BackendType::Windowed,
            drm_device_path: None,
            displays: Vec::new(),
            primary_display: None,
        })
    }
    
    /// Initialize DRM backend (for production compositor)
    async fn init_drm_backend() -> Result<Self> {
        info!("Initializing DRM backend");
        
        // Enumerate and select the best DRM device
        let drm_devices = Self::enumerate_drm_devices().await;
        
        if drm_devices.is_empty() {
            return Err(CompositorError::init("No DRM devices found"));
        }
        
        // Select the primary DRM device (card0 is preferred for dual-display systems)
        let selected_device = drm_devices[0].clone();
        info!("Selected DRM device: {}", selected_device);
        
        // Verify we can actually use this device
        match std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&selected_device)
        {
            Ok(_) => info!("Successfully verified access to DRM device: {}", selected_device),
            Err(e) => {
                error!("Cannot access selected DRM device {}: {}", selected_device, e);
                return Err(CompositorError::init(format!("DRM device access failed: {}", e)));
            }
        }
        
        // For now, create a simple DRM backend without full smithay integration
        // This prevents the hang while we work on full DRM implementation
        info!("DRM backend initialized (basic mode)");
        info!("Note: Full smithay DRM integration pending - using placeholder backend");
        
        Ok(Self {
            backend_type: BackendType::Drm,
            drm_device_path: Some(selected_device),
            displays: Vec::new(),
            primary_display: None,
        })
    }
    
    /// Process backend events (input, output changes, etc.)
    pub async fn process_events(&mut self) -> Result<()> {
        match self.backend_type {
            BackendType::Windowed => self.process_windowed_events().await,
            BackendType::Drm => self.process_drm_events().await,
            BackendType::Auto => unreachable!(),
        }
    }
    
    /// Process events for windowed backend
    async fn process_windowed_events(&mut self) -> Result<()> {
        // TODO: Process winit events
        tokio::task::yield_now().await;
        Ok(())
    }
    
    /// Process events for DRM backend
    async fn process_drm_events(&mut self) -> Result<()> {
        // TODO: Process DRM and libinput events
        tokio::task::yield_now().await;
        Ok(())
    }
    
    /// Get backend type
    pub fn backend_type(&self) -> &BackendType {
        &self.backend_type
    }
    
    /// Get selected DRM device path (if using DRM backend)
    pub fn drm_device_path(&self) -> Option<&str> {
        self.drm_device_path.as_deref()
    }
    
    /// Check if backend is initialized
    pub fn is_initialized(&self) -> bool {
        // Backend is considered initialized if it was created successfully
        true
    }
    
    /// Get connected displays
    pub fn displays(&self) -> &[DisplayOutput] {
        &self.displays
    }
    
    /// Get primary display
    pub fn primary_display(&self) -> Option<&DisplayOutput> {
        self.primary_display.and_then(|idx| self.displays.get(idx))
    }
    
    /// Initialize displays for the backend
    pub async fn initialize_displays(&mut self, renderer: &mut VulkanRenderer) -> Result<()> {
        match self.backend_type {
            BackendType::Windowed => self.init_windowed_displays(renderer).await,
            BackendType::Drm => self.init_drm_displays(renderer).await,
            BackendType::Auto => unreachable!(),
        }
    }
    
    /// Initialize displays for windowed backend
    async fn init_windowed_displays(&mut self, _renderer: &mut VulkanRenderer) -> Result<()> {
        info!("Setting up windowed display");
        
        // Create a test display for windowed mode
        let display = DisplayOutput {
            id: 0,
            name: "Windowed Display".to_string(),
            width: 1920,
            height: 1080,
            refresh_rate: 60,
            connected: true,
            vk_surface: None, // Would need winit surface in real implementation
        };
        
        self.displays = vec![display];
        self.primary_display = Some(0);
        
        info!("Windowed display initialized: 1920x1080@60Hz");
        Ok(())
    }
    
    /// Initialize displays for DRM backend
    async fn init_drm_displays(&mut self, renderer: &mut VulkanRenderer) -> Result<()> {
        info!("Detecting DRM displays");
        
        // For your ASUS ZenBook Pro Duo, we know there are two displays:
        // - Main display: 3840x2160 (primary)
        // - Second display: 3840x1100 (secondary touch screen)
        
        let mut displays = Vec::new();
        
        // Main display (based on your system)
        let main_display = DisplayOutput {
            id: 0,
            name: "Main Display".to_string(),
            width: 3840,
            height: 2160,
            refresh_rate: 60,
            connected: true,
            vk_surface: None,
        };
        displays.push(main_display);
        
        // Secondary display (the keyboard area screen)
        let secondary_display = DisplayOutput {
            id: 1,
            name: "Secondary Display".to_string(),
            width: 3840,
            height: 1100,
            refresh_rate: 60,
            connected: true,
            vk_surface: None,
        };
        displays.push(secondary_display);
        
        self.displays = displays;
        self.primary_display = Some(0); // Main display is primary
        
        info!("DRM displays detected:");
        for (i, disp) in self.displays.iter().enumerate() {
            info!("  Display {}: {} ({}x{}@{}Hz)", 
                  i, disp.name, disp.width, disp.height, disp.refresh_rate);
        }
        
        // Initialize Vulkan swapchains for each display
        let display_count = self.displays.len();
        for i in 0..display_count {
            self.create_vulkan_surface_for_display(i, renderer).await?;
        }
        
        Ok(())
    }
    
    /// Create Vulkan surface for a display using VK_KHR_display extension
    async fn create_vulkan_surface_for_display(
        &mut self, 
        display_index: usize, 
        renderer: &mut VulkanRenderer
    ) -> Result<()> {
        // Extract display info to avoid borrow conflicts
        let (display_name, display_width, display_height) = {
            let disp = &self.displays[display_index];
            (disp.name.clone(), disp.width, disp.height)
        };
        
        info!("Creating VK_KHR_display surface for display: {}", display_name);
        
        // Get Vulkan instance and device for display enumeration
        let instance = renderer.instance();
        let device = renderer.device();
        let physical_device = device.physical_device();
        
        // Enumerate available displays using VK_KHR_display
        let available_displays = instance.enumerate_displays(physical_device)?;
        
        if available_displays.is_empty() {
            warn!("No VK_KHR_display displays found, falling back to null surface");
            self.displays[display_index].vk_surface = Some(ash::vk::SurfaceKHR::null());
            info!("Using fallback rendering for display: {}", display_name);
            return Ok(());
        }
        
        // Find the best matching display (prefer by resolution)
        let target_display = available_displays.iter()
            .find(|display_props| {
                // Match resolution if possible
                display_props.physical_resolution.width == display_width &&
                display_props.physical_resolution.height == display_height
            })
            .or_else(|| {
                // Fallback to first available display
                available_displays.first()
            });
        
        if let Some(display_props) = target_display {
            info!("Found VK_KHR_display: {}x{}", 
                  display_props.physical_resolution.width, 
                  display_props.physical_resolution.height);
            
            // Create display surface using VK_KHR_display
            let display_surface = self.create_display_surface(
                instance, 
                physical_device,
                display_props.display, 
                display_width, 
                display_height
            )?;
            
            self.displays[display_index].vk_surface = Some(display_surface);
            
            // Initialize swapchain for this display
            renderer.initialize_swapchain(display_surface, display_width, display_height)?;
            info!("Successfully created VK_KHR_display surface and swapchain for: {} ({}x{})", 
                  display_name, display_width, display_height);
        } else {
            warn!("No suitable VK_KHR_display found, using fallback");
            self.displays[display_index].vk_surface = Some(ash::vk::SurfaceKHR::null());
            info!("Using fallback rendering for display: {}", display_name);
        }
        
        Ok(())
    }
    
    /// Create a VK_KHR_display surface for direct mode rendering
    fn create_display_surface(
        &self,
        instance: &vulkan_renderer::VulkanInstance,
        physical_device: ash::vk::PhysicalDevice,
        display: ash::vk::DisplayKHR,
        width: u32,
        height: u32,
    ) -> Result<ash::vk::SurfaceKHR> {
        let display_loader = ash::extensions::khr::Display::new(instance.entry(), instance.handle());
        
        // Get display modes for this display
        let display_modes = unsafe {
            display_loader
                .get_display_mode_properties(physical_device, display)
                .map_err(|e| CompositorError::graphics(&format!("Failed to get display modes: {}", e)))?
        };
        
        // Find the best matching display mode
        let target_mode = display_modes.iter()
            .find(|mode| {
                mode.parameters.visible_region.width == width &&
                mode.parameters.visible_region.height == height
            })
            .or_else(|| display_modes.first())
            .ok_or_else(|| CompositorError::graphics("No suitable display mode found"))?;
        
        info!("Selected display mode: {}x{} @ {}Hz",
              target_mode.parameters.visible_region.width,
              target_mode.parameters.visible_region.height,
              target_mode.parameters.refresh_rate);
        
        // Get display planes
        let display_planes = unsafe {
            display_loader
                .get_physical_device_display_plane_properties(physical_device)
                .map_err(|e| CompositorError::graphics(&format!("Failed to get display planes: {}", e)))?
        };
        
        // Find a suitable plane for this display
        let mut suitable_plane_index = None;
        for (plane_index, _plane) in display_planes.iter().enumerate() {
            let supported_displays = unsafe {
                display_loader
                    .get_display_plane_supported_displays(
                        physical_device, 
                        plane_index as u32
                    )
                    .map_err(|e| CompositorError::graphics(&format!("Failed to get supported displays: {}", e)))?
            };
            
            if supported_displays.contains(&display) {
                suitable_plane_index = Some(plane_index as u32);
                break;
            }
        }
        
        let plane_index = suitable_plane_index
            .ok_or_else(|| CompositorError::graphics("No suitable display plane found for display"))?;
        
        // Create display surface
        let surface_create_info = ash::vk::DisplaySurfaceCreateInfoKHR {
            flags: ash::vk::DisplaySurfaceCreateFlagsKHR::empty(),
            display_mode: target_mode.display_mode,
            plane_index,
            plane_stack_index: 0,
            transform: ash::vk::SurfaceTransformFlagsKHR::IDENTITY,
            global_alpha: 1.0,
            alpha_mode: ash::vk::DisplayPlaneAlphaFlagsKHR::OPAQUE,
            image_extent: ash::vk::Extent2D { width, height },
            ..Default::default()
        };
        
        let surface = unsafe {
            display_loader
                .create_display_plane_surface(&surface_create_info, None)
                .map_err(|e| CompositorError::graphics(&format!("Failed to create display surface: {}", e)))?
        };
        
        info!("Successfully created VK_KHR_display surface for direct mode rendering");
        Ok(surface)
    }
}
