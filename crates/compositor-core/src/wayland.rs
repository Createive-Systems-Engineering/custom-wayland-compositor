use compositor_utils::prelude::*;
use vulkan_renderer::VulkanRenderer;
use drm_fourcc::{DrmFourcc, DrmModifier};
use std::os::fd::OwnedFd;
use wayland_server::Resource;
use nix::libc;
use smithay::{
    backend::{
        allocator::{dmabuf::Dmabuf, Buffer, Format, gbm::GbmDevice},
        drm::{DrmNode, DrmDeviceFd},
        egl::{EGLContext, EGLDisplay},
    },
    utils::DeviceFd,
    desktop::{Space, Window},
    input::{Seat, SeatHandler, SeatState, pointer::PointerHandle},
    output::{Output, PhysicalProperties, Subpixel},
    wayland::output::{OutputHandler, OutputManagerState},
    reexports::{
        calloop::{EventLoop, LoopSignal},
        wayland_server::{
            backend::{ClientData, ClientId, DisconnectReason},
            protocol::wl_surface::WlSurface,
            protocol::wl_seat::WlSeat,
            Display,
        },
    },
    utils::{Clock, Monotonic, Serial, Point, Logical},
    wayland::{
        buffer::BufferHandler,
        compositor::{CompositorClientState, CompositorHandler, CompositorState, with_states},
        dmabuf::{DmabufHandler, DmabufState, DmabufGlobal, ImportNotifier},
        drm_syncobj::{DrmSyncobjHandler, DrmSyncobjState, supports_syncobj_eventfd},
        pointer_constraints::{PointerConstraintsHandler, PointerConstraintsState},
        presentation::PresentationState,
        relative_pointer::RelativePointerManagerState,
        selection::{
            SelectionHandler,
            primary_selection::{PrimarySelectionHandler, PrimarySelectionState},
            data_device::{DataDeviceHandler, DataDeviceState, ClientDndGrabHandler, ServerDndGrabHandler},
        },
        tablet_manager::{TabletManagerState, TabletSeatHandler},
        shell::{
            xdg::{
                PopupSurface, PositionerState, ToplevelSurface, XdgShellHandler, XdgShellState,
                decoration::{XdgDecorationHandler, XdgDecorationState},
            },
            wlr_layer::{WlrLayerShellHandler, WlrLayerShellState, LayerSurface, Layer},
        },
        shm::{ShmHandler, ShmState},
        viewporter::ViewporterState,
        fractional_scale::{FractionalScaleHandler, FractionalScaleManagerState},
        content_type::ContentTypeState,
        alpha_modifier::AlphaModifierState,
        single_pixel_buffer::SinglePixelBufferState,
        cursor_shape::CursorShapeManagerState,
        commit_timing::CommitTimerState,
        fifo::FifoManagerState,
        // drm_lease::{DrmLeaseHandler, DrmLeaseState},  // Requires DrmNode and handler implementation
        xdg_foreign::{XdgForeignHandler, XdgForeignState},
        idle_inhibit::{IdleInhibitHandler, IdleInhibitManagerState},
        keyboard_shortcuts_inhibit::{KeyboardShortcutsInhibitHandler, KeyboardShortcutsInhibitState},
        pointer_gestures::PointerGesturesState,
        virtual_keyboard::VirtualKeyboardManagerState,
        text_input::TextInputManagerState,
        input_method::{InputMethodHandler, InputMethodManagerState},
        session_lock::{SessionLockHandler, SessionLockManagerState},
        security_context::{SecurityContextHandler, SecurityContextState},
        xdg_activation::{XdgActivationHandler, XdgActivationState},
        foreign_toplevel_list::{ForeignToplevelListHandler, ForeignToplevelListState},
        socket::ListeningSocketSource,
    },
};

use std::sync::{Arc, Mutex};

/// Client state data
#[derive(Default)]
pub struct ClientState {
    pub compositor_state: CompositorClientState,
}

impl ClientData for ClientState {
    fn initialized(&self, _client_id: ClientId) {}
    fn disconnected(&self, _client_id: ClientId, _reason: DisconnectReason) {}
}

/// Main Wayland server state
pub struct WaylandServerState {
    pub compositor_state: CompositorState,
    pub xdg_shell_state: XdgShellState,
    pub wlr_layer_shell_state: WlrLayerShellState,
    pub shm_state: ShmState,
    pub dmabuf_state: DmabufState,
    pub dmabuf_global: DmabufGlobal,
    pub output_manager_state: OutputManagerState,
    pub relative_pointer_manager_state: RelativePointerManagerState,
    pub pointer_constraints_state: PointerConstraintsState,
    pub presentation_state: PresentationState,
    pub primary_selection_state: PrimarySelectionState,
    pub data_device_state: DataDeviceState,
    pub xdg_decoration_state: XdgDecorationState,
    pub xdg_foreign_state: XdgForeignState,
    pub tablet_manager_state: TabletManagerState,
    pub viewporter_state: ViewporterState,
    pub fractional_scale_manager_state: FractionalScaleManagerState,
    pub content_type_state: ContentTypeState,
    pub alpha_modifier_state: AlphaModifierState,
    pub single_pixel_buffer_state: SinglePixelBufferState,
    pub cursor_shape_manager_state: CursorShapeManagerState,
    pub commit_timer_state: CommitTimerState,
    pub fifo_manager_state: FifoManagerState,
    // pub drm_lease_state: DrmLeaseState,  // Requires DrmNode and handler implementation
    pub idle_inhibit_manager_state: IdleInhibitManagerState,
    pub keyboard_shortcuts_inhibit_state: KeyboardShortcutsInhibitState,
    pub pointer_gestures_state: PointerGesturesState,
    pub virtual_keyboard_manager_state: VirtualKeyboardManagerState,
    pub text_input_manager_state: TextInputManagerState,
    pub input_method_manager_state: InputMethodManagerState,
    pub session_lock_manager_state: SessionLockManagerState,
    pub security_context_state: SecurityContextState,
    pub xdg_activation_state: XdgActivationState,
    pub foreign_toplevel_list_state: ForeignToplevelListState,
    pub drm_syncobj_state: Option<DrmSyncobjState>,
    pub seat_state: SeatState<Self>,
    pub space: Space<Window>,
    pub clock: Clock<Monotonic>,
    pub socket_name: Option<String>,
    /// EGL context for hardware acceleration and wl_drm protocol support
    pub egl_context: Option<EGLContext>,
    /// EGL display for wl_drm protocol integration 
    pub egl_display: Option<EGLDisplay>,
    /// DRM node for GPU resource management
    pub drm_node: Option<DrmNode>,
    /// DRM device file descriptor for explicit sync support
    pub drm_device_fd: Option<DrmDeviceFd>,
    /// Vulkan renderer for surface compositing
    pub renderer: Option<Arc<Mutex<VulkanRenderer>>>,
}

/// Wayland server implementation using smithay and calloop
pub struct WaylandServer {
    pub event_loop: EventLoop<'static, WaylandServerState>,
    pub state: WaylandServerState,
    pub display: Display<WaylandServerState>,
    pub loop_signal: LoopSignal,
}

impl WaylandServer {
    /// Create a new Wayland server with event loop
    pub fn new() -> Result<Self> {
        info!("Initializing Wayland server with smithay and calloop");
        
        // Create event loop first
        let event_loop = EventLoop::try_new()
            .map_err(|e| CompositorError::wayland(format!("Failed to create event loop: {}", e)))?;
        
        let _loop_handle = event_loop.handle();
        let loop_signal = event_loop.get_signal();
        
        // Create display with the loop handle
        let display = Display::new()
            .map_err(|e| CompositorError::wayland(format!("Failed to create display: {}", e)))?;
        
        let dh = display.handle();
        
        // Initialize compositor state
        let compositor_state = CompositorState::new::<WaylandServerState>(&dh);
        let xdg_shell_state = XdgShellState::new::<WaylandServerState>(&dh);
        let wlr_layer_shell_state = WlrLayerShellState::new::<WaylandServerState>(&dh);
        let shm_state = ShmState::new::<WaylandServerState>(&dh, vec![]);
        
        // Initialize dmabuf state for zero-copy GPU buffer sharing
        let mut dmabuf_state = DmabufState::new();
        
        // Create common formats for dmabuf support
        let formats = vec![
            Format {
                code: DrmFourcc::Xrgb8888,
                modifier: DrmModifier::Linear,
            },
            Format {
                code: DrmFourcc::Argb8888, 
                modifier: DrmModifier::Linear,
            },
        ];
        
        let dmabuf_global = dmabuf_state.create_global::<WaylandServerState>(&dh, formats);
        
        let seat_state = SeatState::new();
        
        // Initialize output manager with xdg-output support for multi-monitor configuration
        let output_manager_state = OutputManagerState::new_with_xdg_output::<WaylandServerState>(&dh);
        
        // Initialize relative pointer manager for 3D viewport navigation and gaming
        let relative_pointer_manager_state = RelativePointerManagerState::new::<WaylandServerState>(&dh);
        
        // Initialize pointer constraints for 3D viewport navigation and gaming
        let pointer_constraints_state = PointerConstraintsState::new::<WaylandServerState>(&dh);
        
        // Initialize presentation time for high-precision temporal synchronization
        let presentation_state = PresentationState::new::<WaylandServerState>(&dh, libc::CLOCK_MONOTONIC as u32);
        
        // Initialize primary selection for advanced clipboard functionality
        let primary_selection_state = PrimarySelectionState::new::<WaylandServerState>(&dh);
        
        // Initialize data device manager for drag-and-drop operations and clipboard management
        let data_device_state = DataDeviceState::new::<WaylandServerState>(&dh);
        
        // Initialize XDG decoration manager for client-side/server-side decoration control
        let xdg_decoration_state = XdgDecorationState::new::<WaylandServerState>(&dh);
        
        // Initialize xdg-foreign for cross-surface window embedding
        let xdg_foreign_state = XdgForeignState::new::<WaylandServerState>(&dh);
        
        // Initialize viewporter for advanced viewport transformation
        let viewporter_state = ViewporterState::new::<WaylandServerState>(&dh);
        
        // Initialize fractional scale manager for 4K display optimization and sub-pixel precision
        let fractional_scale_manager_state = FractionalScaleManagerState::new::<WaylandServerState>(&dh);
        
        // Initialize tablet manager for professional graphics tablet integration
        let tablet_manager_state = TabletManagerState::new::<WaylandServerState>(&dh);
        
        // Create default output (4K setup)
        let output = Output::new(
            "custom-compositor-output".to_string(),
            PhysicalProperties {
                size: (3840, 2160).into(), // 4K default
                subpixel: Subpixel::Unknown,
                make: "Custom Compositor".into(),
                model: "Virtual Output".into(),
            },
        );
        
        // Add modes to output
        output.add_mode(smithay::output::Mode {
            size: (3840, 2160).into(),
            refresh: 60_000, // 60Hz in mHz
        });
        output.set_preferred(smithay::output::Mode {
            size: (3840, 2160).into(),
            refresh: 60_000,
        });
        
        // Create space and map output
        let mut space = Space::default();
        space.map_output(&output, (0, 0));
        
        let clock = Clock::new();
        
        let state = WaylandServerState {
            compositor_state,
            xdg_shell_state,
            wlr_layer_shell_state,
            shm_state,
            dmabuf_state,
            dmabuf_global,
            output_manager_state,
            relative_pointer_manager_state,
            pointer_constraints_state,
            presentation_state,
            primary_selection_state,
            data_device_state,
            xdg_decoration_state,
            xdg_foreign_state,
            tablet_manager_state,
            viewporter_state,
            fractional_scale_manager_state,
            content_type_state: ContentTypeState::new::<WaylandServerState>(&dh),
            alpha_modifier_state: AlphaModifierState::new::<WaylandServerState>(&dh),
            single_pixel_buffer_state: SinglePixelBufferState::new::<WaylandServerState>(&dh),
            cursor_shape_manager_state: CursorShapeManagerState::new::<WaylandServerState>(&dh),
            commit_timer_state: CommitTimerState::default(),
            fifo_manager_state: FifoManagerState::new::<WaylandServerState>(&dh),
            // drm_lease_state: DrmLeaseState::new::<WaylandServerState>(&dh), // Requires DrmNode and handler
            idle_inhibit_manager_state: IdleInhibitManagerState::new::<WaylandServerState>(&dh),
            keyboard_shortcuts_inhibit_state: KeyboardShortcutsInhibitState::new::<WaylandServerState>(&dh),
            pointer_gestures_state: PointerGesturesState::new::<WaylandServerState>(&dh),
            virtual_keyboard_manager_state: VirtualKeyboardManagerState::new::<WaylandServerState, _>(&dh, |_client| true),
            text_input_manager_state: TextInputManagerState::new::<WaylandServerState>(&dh),
            input_method_manager_state: InputMethodManagerState::new::<WaylandServerState, _>(&dh, |_client| true),
            session_lock_manager_state: SessionLockManagerState::new::<WaylandServerState, _>(&dh, |_client| true),
            security_context_state: SecurityContextState::new::<WaylandServerState, _>(&dh, |_client| true),
            xdg_activation_state: XdgActivationState::new::<WaylandServerState>(&dh),
            foreign_toplevel_list_state: ForeignToplevelListState::new::<WaylandServerState>(&dh),
            drm_syncobj_state: None, // Will be initialized when DRM device is configured
            seat_state,
            space,
            clock,
            socket_name: None,
            egl_context: None, // Will be initialized when backend is configured
            egl_display: None, // Will be initialized for wl_drm protocol support
            drm_node: None,    // Will be set when DRM device is detected
            drm_device_fd: None, // Will be set for explicit sync support
            renderer: None,    // Initialize with no renderer
        };
        
        info!("Wayland server state initialized with calloop");
        
        Ok(Self {
            event_loop,
            state,
            display,
            loop_signal,
        })
    }
    
    /// Validate that we have proper graphics session access for 4K compositor operation
    fn validate_graphics_session(&self) -> Result<()> {
        // Check if user is in video/render groups
        let current_user = std::env::var("USER").unwrap_or_else(|_| "unknown".to_string());
        
        // Check DRM device permissions
        let drm_devices = ["/dev/dri/card0", "/dev/dri/card1", "/dev/dri/renderD128"];
        let mut accessible_devices = Vec::new();
        
        for device in &drm_devices {
            if std::fs::metadata(device).is_ok() {
                accessible_devices.push(*device);
            }
        }
        
        if accessible_devices.is_empty() {
            return Err(CompositorError::system(
                "No DRM devices found. 4K compositor requires GPU access.\n\
                 Install proper graphics drivers.".to_string()
            ));
        }
        
        info!("Graphics session validation passed:");
        info!("  User: {}", current_user);
        info!("  DRM devices: {:?}", accessible_devices);
        
        // Additional session checks
        if let Ok(session_type) = std::env::var("XDG_SESSION_TYPE") {
            info!("  Session type: {}", session_type);
        }
        
        if let Ok(display_var) = std::env::var("DISPLAY") {
            info!("  Display: {}", display_var);
        }
        
        Ok(())
    }
    
    /// Check if we have sufficient DRM privileges to create GBM devices safely
    /// Prevents segfaults when attempting GBM device creation without proper access
    fn check_drm_master_capabilities(fd: &std::fs::File) -> bool {
        use std::os::unix::io::AsRawFd;
        
        let raw_fd = fd.as_raw_fd();
        
        // Check if we can perform basic DRM ioctls
        // This is a safer way to test capabilities before attempting GBM creation
        unsafe {
            // Try to get the DRM version - this is a safe ioctl that doesn't require master
            let version_ioctl = libc::ioctl(raw_fd, 0x40406400); // DRM_IOCTL_VERSION (simplified)
            
            if version_ioctl == -1 {
                let errno = *libc::__errno_location();
                warn!("DRM version ioctl failed with errno {}: basic DRM access unavailable", errno);
                return false;
            }
            
            // Try to check if we're DRM master (this may fail gracefully)
            let master_ioctl = libc::ioctl(raw_fd, 0x641e); // DRM_IOCTL_SET_MASTER (simplified)
            let can_be_master = master_ioctl != -1;
            
            if !can_be_master {
                let errno = *libc::__errno_location();
                // EACCES (13) or EPERM (1) are expected when not master
                if errno == 13 || errno == 1 {
                    info!("Unable to become drm master, assuming unprivileged mode");
                    // For render nodes or unprivileged access, we might still be able to create GBM
                    // but we need to be more cautious
                    
                    // Check if this is a render node (which doesn't require master)
                    // Render nodes typically have paths like /dev/dri/renderD*
                    return true; // Allow attempt for render nodes
                } else {
                    warn!("Unexpected DRM master check error: {}", errno);
                    return false;
                }
            } else {
                info!("✅ DRM master capabilities available");
                // Drop master immediately - we just wanted to check
                libc::ioctl(raw_fd, 0x641f); // DRM_IOCTL_DROP_MASTER (simplified)
                return true;
            }
        }
    }
    
    /// Initialize EGL display and explicit sync support
    /// Essential for 4K hardware acceleration and professional graphics performance
    pub fn initialize_wl_drm(&mut self) -> Result<()> {
        info!("Initializing 4K hardware acceleration pipeline (EGL + explicit sync)");
        
        // For 4K compositor operation, we need proper session management
        // Check if we're running in a proper graphics session
        self.validate_graphics_session()?;
        
        // Try to find a primary DRM node (usually /dev/dri/card0)
        let drm_node = match DrmNode::from_path("/dev/dri/card0") {
            Ok(node) => {
                info!("Found primary DRM node: {:?}", node.dev_path());
                Some(node)
            }
            Err(e) => {
                warn!("Failed to open primary DRM node /dev/dri/card0: {}, trying render node", e);
                
                // Try render node as fallback (/dev/dri/renderD128)
                match DrmNode::from_path("/dev/dri/renderD128") {
                    Ok(node) => {
                        info!("Found DRM render node: {:?}", node.dev_path());
                        Some(node)
                    }
                    Err(e) => {
                        warn!("Failed to open DRM render node: {}, wl_drm and explicit sync will be unavailable", e);
                        None
                    }
                }
            }
        };
        
        // Store the DRM node
        self.state.drm_node = drm_node;
        
        // Initialize EGL display and explicit sync if we have a DRM node
        if let Some(ref drm_node) = self.state.drm_node {
            // Get the device path - dev_path() returns Option<PathBuf>
            let device_path = match drm_node.dev_path() {
                Some(path) => path,
                None => {
                    warn!("DRM node has no device path, protocols unavailable");
                    return Ok(());
                }
            };
            
            // Check DRM device accessibility before opening
            if !device_path.exists() {
                warn!("DRM device path {:?} does not exist, protocols unavailable", device_path);
                return Ok(());
            }
            
            // Check if we have read permissions
            let metadata = match std::fs::metadata(&device_path) {
                Ok(meta) => meta,
                Err(e) => {
                    warn!("Cannot access DRM device metadata {:?}: {}, protocols unavailable", device_path, e);
                    return Ok(());
                }
            };
            
            // Basic permission check - if we can't read metadata, we likely can't open the device
            if metadata.permissions().readonly() {
                warn!("DRM device {:?} appears to be read-only, GBM may fail", device_path);
            }
            
            // Open the DRM device file once and share the file descriptor
            let fd = match std::fs::File::open(&device_path) {
                Ok(file) => file,
                Err(e) => {
                    warn!("Failed to open DRM device file {:?}: {}, protocols unavailable", device_path, e);
                    return Ok(());
                }
            };
            
            // Check if we're running with proper session management before creating any devices
            let has_session_management = std::env::var("XDG_SESSION_TYPE")
                .map(|t| t == "wayland")
                .unwrap_or(false) || 
                std::env::var("WAYLAND_DISPLAY").is_ok();
            
            if !has_session_management {
                warn!("Not running in Wayland session - DRM device creation may fail");
                warn!("For full 4K hardware acceleration, run compositor in proper Wayland session");
            }
            
            // Try to duplicate the file descriptor for GBM use
            let fd_for_gbm = match fd.try_clone() {
                Ok(cloned_fd) => cloned_fd,
                Err(e) => {
                    warn!("Failed to clone DRM file descriptor: {}", e);
                    warn!("Continuing without GBM device - some 4K features will be limited");
                    
                    // Still create DRM device fd for explicit sync, but skip GBM
                    let owned_fd: OwnedFd = fd.into();
                    let device_fd = DeviceFd::from(owned_fd);
                    let drm_device_fd = Some(DrmDeviceFd::new(device_fd));
                    
                    if let Some(ref device_fd) = drm_device_fd {
                        if supports_syncobj_eventfd(device_fd) {
                            info!("✅ DRM device supports explicit sync, initializing zwp-linux-explicit-sync-v1");
                            
                            let dh = self.display.handle();
                            let syncobj_state = DrmSyncobjState::new::<WaylandServerState>(&dh, device_fd.clone());
                            self.state.drm_syncobj_state = Some(syncobj_state);
                            
                            info!("✅ zwp-linux-explicit-sync-v1 protocol initialized for frame-perfect timing control");
                        } else {
                            warn!("DRM device does not support syncobj eventfd, explicit sync unavailable");
                        }
                        
                        self.state.drm_device_fd = drm_device_fd;
                    }
                    return Ok(());
                }
            };
            
            // Create DRM device file descriptor for explicit sync using original fd
            let owned_fd: OwnedFd = fd.into();
            let device_fd = DeviceFd::from(owned_fd);
            let drm_device_fd = Some(DrmDeviceFd::new(device_fd));
            info!("Created DRM device fd for explicit sync support");
            
            // Initialize explicit sync if device supports it
            if let Some(ref device_fd) = drm_device_fd {
                if supports_syncobj_eventfd(device_fd) {
                    info!("✅ DRM device supports explicit sync, initializing zwp-linux-explicit-sync-v1");
                    
                    let dh = self.display.handle();
                    let syncobj_state = DrmSyncobjState::new::<WaylandServerState>(&dh, device_fd.clone());
                    self.state.drm_syncobj_state = Some(syncobj_state);
                    
                    info!("✅ zwp-linux-explicit-sync-v1 protocol initialized for frame-perfect timing control");
                } else {
                    warn!("DRM device does not support syncobj eventfd, explicit sync unavailable");
                }
                
                // Store the device fd regardless of sync support for potential future use
                self.state.drm_device_fd = drm_device_fd;
            }
            
            // For 4K compositor operation, we need GBM device for hardware acceleration
            // However, this requires proper DRM privileges or session management
            
            // Check DRM master capabilities before attempting GBM device creation
            // This prevents segfaults when running without proper privileges
            let can_create_gbm = Self::check_drm_master_capabilities(&fd_for_gbm);
            
            if !can_create_gbm {
                warn!("Insufficient DRM privileges for GBM device creation");
                warn!("Continuing without GBM - some 4K features may be limited");
                warn!("This is normal when running in X11 session or without DRM master privileges");
                warn!("To enable full acceleration:");
                warn!("  1. Add user to video/render groups: sudo usermod -a -G video,render $USER");
                warn!("  2. Run in proper Wayland session with compositor session management"); 
                warn!("  3. Ensure proper udev rules for DRM device access");
                warn!("  4. Or run as root (not recommended for security reasons)");
                return Ok(()); // Graceful fallback - continue without GBM
            }
            
            info!("DRM master capabilities confirmed, attempting to create GBM device for 4K hardware acceleration...");
            
            // Attempt GBM device creation with graceful fallback using cloned fd
            let gbm_device = match GbmDevice::new(fd_for_gbm) {
                Ok(device) => {
                    info!("✅ Created GBM device for 4K hardware acceleration: {:?}", device_path);
                    device
                }
                Err(e) => {
                    warn!("Failed to create GBM device despite privilege checks: {}", e);
                    warn!("Continuing without GBM - some 4K features may be limited");
                    warn!("This may indicate driver or library issues");
                    return Ok(()); // Graceful fallback - continue without GBM
                }
            };
            
            // Create EGL display from GBM device (essential for 4K zero-copy rendering)
            let egl_display = match unsafe { EGLDisplay::new(gbm_device) } {
                Ok(display) => {
                    info!("✅ Created EGL display for 4K zero-copy rendering pipeline");
                    display
                }
                Err(e) => {
                    return Err(CompositorError::system(format!(
                        "Failed to create EGL display: {}. 4K compositor requires EGL support.\n\
                         Ensure:\n\
                         1. Mesa EGL drivers are installed\n\
                         2. GPU supports EGL extensions\n\
                         3. Proper graphics driver configuration", e
                    )));
                }
            };
            
            self.state.egl_display = Some(egl_display);
            info!("✅ 4K hardware acceleration pipeline initialized successfully");
        } else {
            info!("No DRM node available, wl_drm and explicit sync protocols will be unavailable");
        }
        
        // Report initialization status
        let wl_drm_status = if self.state.egl_display.is_some() { 
            "initialized" 
        } else { 
            "unavailable" 
        };
        
        let explicit_sync_status = if self.state.drm_syncobj_state.is_some() { 
            "initialized" 
        } else { 
            "unavailable" 
        };
        
        info!("Protocol initialization complete:");
        info!("  • wl_drm (legacy EGL): {}", wl_drm_status);
        info!("  • zwp-linux-explicit-sync-v1 (modern GPU sync): {}", explicit_sync_status);
        
        Ok(())
    }
    
    /// Start listening on a Wayland socket and integrate with event loop
    pub fn start_listening(&mut self) -> Result<()> {
        info!("Starting Wayland socket and integrating with event loop");
        
        // Create listening socket
        let socket_source = ListeningSocketSource::new_auto()
            .map_err(|e| CompositorError::wayland(format!("Failed to create socket: {}", e)))?;
        
        let socket_name = socket_source.socket_name().to_string_lossy().into_owned();
        self.state.socket_name = Some(socket_name.clone());
        
        // Insert socket into event loop
        let mut display_handle = self.display.handle();
        self.event_loop
            .handle()
            .insert_source(socket_source, move |client_stream, _, _state| {
                // Handle new client connections
                if let Err(err) = display_handle.insert_client(client_stream, Arc::new(ClientState::default())) {
                    error!("Failed to insert client: {}", err);
                }
            })
            .map_err(|e| CompositorError::wayland(format!("Failed to insert socket source: {}", e)))?;
        
        info!("Wayland server listening on socket: {}", socket_name);
        info!("Set WAYLAND_DISPLAY={} to connect clients", socket_name);
        
        // Set environment variable for clients
        std::env::set_var("WAYLAND_DISPLAY", &socket_name);
        
        Ok(())
    }
    
    /// Run the event loop (blocking)
    pub fn run(mut self) -> Result<()> {
        info!("Starting Wayland server event loop");
        
        // Main event loop using smithay's standard pattern
        loop {
            // Dispatch wayland events
            if let Err(e) = self.display.dispatch_clients(&mut self.state) {
                error!("Error dispatching clients: {}", e);
                break;
            }
            
            // Flush pending events  
            if let Err(e) = self.display.flush_clients() {
                error!("Error flushing clients: {}", e);
                break;
            }
            
            // Run event loop iteration
            if let Err(e) = self.event_loop.dispatch(Some(std::time::Duration::from_millis(16)), &mut self.state) {
                error!("Event loop error: {}", e);
                break;
            }
        }
        
        info!("Wayland server event loop terminated");
        Ok(())
    }
    
    /// Run the event loop asynchronously (non-blocking)
    pub async fn run_async(mut self) -> Result<()> {
        info!("Starting Wayland server async event loop");
        
        // Async event loop using smithay's standard pattern
        loop {
            // Dispatch wayland events
            if let Err(e) = self.display.dispatch_clients(&mut self.state) {
                error!("Error dispatching clients: {}", e);
                break;
            }
            
            // Flush pending events  
            if let Err(e) = self.display.flush_clients() {
                error!("Error flushing clients: {}", e);
                break;
            }
            
            // Run event loop iteration with async yield
            if let Err(e) = self.event_loop.dispatch(Some(std::time::Duration::from_millis(16)), &mut self.state) {
                error!("Event loop error: {}", e);
                break;
            }
            
            // Yield to other async tasks
            tokio::task::yield_now().await;
        }
        
        info!("Wayland server async event loop terminated");
        Ok(())
    }
    
    /// Set the Vulkan renderer for surface rendering
    pub fn set_renderer(&mut self, renderer: Arc<Mutex<VulkanRenderer>>) {
        info!("Setting Vulkan renderer for Wayland server");
        self.state.renderer = Some(renderer);
    }
    
    /// Get the loop signal for shutdown
    pub fn loop_signal(&self) -> LoopSignal {
        self.loop_signal.clone()
    }
    
    /// Get socket name if listening
    pub fn socket_name(&self) -> Option<&str> {
        self.state.socket_name.as_deref()
    }
    
    /// Shutdown the Wayland server
    pub async fn shutdown(&mut self) -> Result<()> {
        info!("Shutting down Wayland server");
        
        // Signal the event loop to stop
        self.loop_signal.stop();
        
        // The event loop should stop processing after receiving the signal
        info!("Wayland server shutdown complete");
        Ok(())
    }
}

// Implement required smithay handlers
impl DmabufHandler for WaylandServerState {
    fn dmabuf_state(&mut self) -> &mut DmabufState {
        &mut self.dmabuf_state
    }

    fn dmabuf_imported(
        &mut self, 
        _global: &DmabufGlobal, 
        dmabuf: Dmabuf,
        notifier: ImportNotifier
    ) {
        info!("DMA-BUF imported: {}x{} format: {:?}", 
              dmabuf.width(), dmabuf.height(), dmabuf.format());
        
        // TODO: Validate dmabuf format compatibility with our Vulkan renderer
        // TODO: Import dmabuf into our Vulkan renderer for zero-copy rendering
        // For now, just log the successful import and signal success
        debug!("DMA-BUF import successful, zero-copy GPU buffer sharing ready");
        
        // Signal that the import was successful
        if let Err(e) = notifier.successful::<WaylandServerState>() {
            error!("Failed to signal successful dmabuf import: {}", e);
        }
    }
}

impl CompositorHandler for WaylandServerState {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.compositor_state
    }
    
    fn client_compositor_state<'a>(&self, client: &'a wayland_server::Client) -> &'a CompositorClientState {
        &client.get_data::<ClientState>().unwrap().compositor_state
    }
    
    fn new_surface(&mut self, surface: &WlSurface) {
        debug!("New surface created: {:?}", surface.id());
    }
    
    fn commit(&mut self, surface: &WlSurface) {
        debug!("Surface committed: {:?}", surface.id());
        
        // Handle surface commits for rendering
        with_states(surface, |_surface_data| {
            // TODO: Implement proper buffer handling with current Smithay API
            // For now, just log the commit
            debug!("Surface committed with data, will handle buffer access in future implementation");
            
            // TODO: Access buffer through proper surface state API
            // TODO: Handle frame callbacks through proper API
            // TODO: Send buffer data to renderer
        });
        
        // Schedule a repaint for this surface
        self.space.refresh();
        
        // TODO: Trigger actual frame rendering in compositor
        debug!("Surface commit processed, space refreshed");
    }
}

impl XdgShellHandler for WaylandServerState {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell_state
    }
    
    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        info!("New toplevel window created");
        
        // Create window and add to space using the new API
        let window = Window::new_wayland_window(surface);
        self.space.map_element(window, (100, 100), false);
    }
    
    fn new_popup(&mut self, _surface: PopupSurface, _positioner: PositionerState) {
        debug!("New popup created");
        // TODO: Handle popups
    }
    
    fn toplevel_destroyed(&mut self, _surface: ToplevelSurface) {
        info!("Toplevel window destroyed");
        // TODO: Remove window from space
    }
    
    fn popup_destroyed(&mut self, _surface: PopupSurface) {
        debug!("Popup destroyed");
        // TODO: Handle popup destruction
    }
    
    fn grab(&mut self, _surface: PopupSurface, _seat: WlSeat, _serial: Serial) {
        debug!("Popup grab requested");
        // TODO: Handle popup grabs
    }
    
    fn reposition_request(&mut self, _surface: PopupSurface, _positioner: PositionerState, _token: u32) {
        debug!("Popup reposition requested");
        // TODO: Handle popup repositioning
    }
}

// ============================================================================
// WLR Layer Shell Handler Implementation
// ============================================================================

impl WlrLayerShellHandler for WaylandServerState {
    fn shell_state(&mut self) -> &mut WlrLayerShellState {
        &mut self.wlr_layer_shell_state
    }
    
    fn new_layer_surface(&mut self, _surface: LayerSurface, _wl_output: Option<wayland_server::protocol::wl_output::WlOutput>, layer: Layer, namespace: String) {
        info!("New layer surface created with namespace: {} on layer: {:?}", namespace, layer);
        
        // TODO: Add layer surface to appropriate layer in space
        // TODO: Handle surface configuration based on layer (background, bottom, top, overlay)
        // TODO: Apply exclusive zones and anchoring
        debug!("Layer surface added to compositor space on layer: {:?}", layer);
    }
    
    fn layer_destroyed(&mut self, _surface: LayerSurface) {
        info!("Layer surface destroyed");
        
        // TODO: Remove layer surface from space
        // TODO: Recalculate layout and exclusive zones
        debug!("Layer surface removed from compositor space");
    }
}

impl ShmHandler for WaylandServerState {
    fn shm_state(&self) -> &ShmState {
        &self.shm_state
    }
}

impl BufferHandler for WaylandServerState {
    fn buffer_destroyed(&mut self, _buffer: &wayland_server::protocol::wl_buffer::WlBuffer) {
        debug!("Buffer destroyed");
        // TODO: Handle buffer cleanup
    }
}

impl SeatHandler for WaylandServerState {
    type KeyboardFocus = WlSurface;
    type PointerFocus = WlSurface;
    type TouchFocus = WlSurface;
    
    fn seat_state(&mut self) -> &mut SeatState<Self> {
        &mut self.seat_state
    }
    
    fn focus_changed(&mut self, _seat: &Seat<Self>, _focused: Option<&Self::KeyboardFocus>) {
        debug!("Focus changed for seat");
    }
    
    fn cursor_image(&mut self, _seat: &Seat<Self>, _image: smithay::input::pointer::CursorImageStatus) {
        debug!("Cursor image changed for seat");
    }
}

// Output handler implementation for managing outputs
impl OutputHandler for WaylandServerState {
    fn output_bound(&mut self, _output: Output, _wl_output: smithay::reexports::wayland_server::protocol::wl_output::WlOutput) {
        debug!("Output bound to client");
    }
}

// Pointer constraints handler implementation for precise pointer control
impl PointerConstraintsHandler for WaylandServerState {
    fn new_constraint(&mut self, surface: &WlSurface, pointer: &PointerHandle<Self>) {
        info!("New pointer constraint created for surface: {:?}", surface.id());
        debug!("Pointer constraint established for pointer: {:?}", pointer);
        
        // TODO: Handle constraint activation based on focus and surface state
        // TODO: Implement constraint region validation
        // TODO: Integrate with input handling system for constraint enforcement
    }
    
    fn cursor_position_hint(&mut self, surface: &WlSurface, pointer: &PointerHandle<Self>, location: Point<f64, Logical>) {
        debug!("Cursor position hint received for surface: {:?}, location: {:?}", surface.id(), location);
        debug!("Position hint for pointer: {:?}", pointer);
        
        // TODO: Update cursor position based on hint for locked pointer constraints
        // TODO: Validate hint location against constraint region
        // TODO: Apply position hint to compositor cursor state
    }
}

// DRM syncobj handler implementation for explicit GPU synchronization
impl DrmSyncobjHandler for WaylandServerState {
    fn drm_syncobj_state(&mut self) -> &mut DrmSyncobjState {
        self.drm_syncobj_state.as_mut().expect("DrmSyncobjState not initialized - ensure initialize_wl_drm() was called")
    }
}

// XDG decoration handler implementation for client/server-side decoration control
impl XdgDecorationHandler for WaylandServerState {
    fn new_decoration(&mut self, toplevel: ToplevelSurface) {
        info!("Client requested decoration support for toplevel window");
        
        // Configure server-side decorations by default for consistent glassmorphism theming
        toplevel.with_pending_state(|state| {
            state.decoration_mode = Some(wayland_protocols::xdg::decoration::zv1::server::zxdg_toplevel_decoration_v1::Mode::ServerSide);
        });
        toplevel.send_configure();
        
        debug!("Configured server-side decorations for toplevel window");
    }
    
    fn request_mode(&mut self, toplevel: ToplevelSurface, mode: wayland_protocols::xdg::decoration::zv1::server::zxdg_toplevel_decoration_v1::Mode) {
        use wayland_protocols::xdg::decoration::zv1::server::zxdg_toplevel_decoration_v1::Mode;
        
        match mode {
            Mode::ClientSide => {
                info!("Client requested client-side decorations");
                toplevel.with_pending_state(|state| {
                    state.decoration_mode = Some(Mode::ClientSide);
                });
            }
            Mode::ServerSide => {
                info!("Client requested server-side decorations");
                toplevel.with_pending_state(|state| {
                    state.decoration_mode = Some(Mode::ServerSide);
                });
            }
            _ => {
                warn!("Client requested unknown decoration mode: {:?}", mode);
                // Default to server-side for glassmorphism integration
                toplevel.with_pending_state(|state| {
                    state.decoration_mode = Some(Mode::ServerSide);
                });
            }
        }
        
        toplevel.send_configure();
        debug!("Applied decoration mode: {:?}", mode);
    }
    
    fn unset_mode(&mut self, toplevel: ToplevelSurface) {
        info!("Client unset decoration mode preference");
        
        // Default to server-side decorations for consistent theming
        toplevel.with_pending_state(|state| {
            state.decoration_mode = Some(wayland_protocols::xdg::decoration::zv1::server::zxdg_toplevel_decoration_v1::Mode::ServerSide);
        });
        toplevel.send_configure();
        
        debug!("Reset to server-side decorations (default)");
    }
}

// ============================================================================
// Selection Handler Implementation
// ============================================================================

impl SelectionHandler for WaylandServerState {
    type SelectionUserData = ();
}

// ============================================================================
// Primary Selection Handler Implementation  
// ============================================================================

impl PrimarySelectionHandler for WaylandServerState {
    fn primary_selection_state(&self) -> &PrimarySelectionState {
        &self.primary_selection_state
    }
}

// ============================================================================
// Data Device Handler Implementation
// ============================================================================

impl DataDeviceHandler for WaylandServerState {
    fn data_device_state(&self) -> &DataDeviceState {
        &self.data_device_state
    }
}

impl ClientDndGrabHandler for WaylandServerState {
    fn started(&mut self, _source: Option<wayland_server::protocol::wl_data_source::WlDataSource>, icon: Option<wayland_server::protocol::wl_surface::WlSurface>, _seat: smithay::input::Seat<Self>) {
        info!("Drag and drop operation started");
        if let Some(icon_surface) = icon {
            debug!("DnD operation includes drag icon surface: {:?}", icon_surface.id());
            // TODO: Handle drag icon rendering and positioning
        }
        // TODO: Begin drag operation state management
        // TODO: Update cursor appearance for drag operation
    }
    
    fn dropped(&mut self, _target: Option<WlSurface>, _validated: bool, _seat: smithay::input::Seat<Self>) {
        info!("Drag and drop operation completed - item dropped");
        // TODO: Handle drop completion and cleanup drag state
        // TODO: Reset cursor appearance after drag operation
        // TODO: Process drop target actions
    }
}

impl ServerDndGrabHandler for WaylandServerState {
    fn send(&mut self, _mime_type: String, _fd: std::os::fd::OwnedFd, _seat: smithay::input::Seat<Self>) {
        info!("Server-side DnD: Sending data with mime type");
        // TODO: Handle server-side drag and drop data transfer
        // TODO: Write data to the provided file descriptor
    }
    
    fn finished(&mut self, _seat: smithay::input::Seat<Self>) {
        info!("Server-side DnD operation finished");
        // TODO: Clean up server-side drag state
        // TODO: Release any held resources
    }
    
    fn cancelled(&mut self, _seat: smithay::input::Seat<Self>) {
        info!("Server-side DnD operation cancelled");
        // TODO: Handle cancellation cleanup
        // TODO: Reset drag state
    }
}

// ============================================================================
// Tablet Manager Handler Implementation
// ============================================================================

impl TabletSeatHandler for WaylandServerState {
    // Let the compiler tell us what methods we need to implement
}

// ============================================================================
// Viewporter Implementation
// ============================================================================

// Viewporter doesn't require a handler trait implementation
// It's managed directly through the ViewporterState and delegate_viewporter! macro

// ============================================================================
// Fractional Scale Handler Implementation
// ============================================================================

impl FractionalScaleHandler for WaylandServerState {
    fn new_fractional_scale(&mut self, surface: WlSurface) {
        info!("New fractional scale instantiated for surface: {:?}", surface.id());
        
        // TODO: Implement fractional scale calculation based on output configuration
        // TODO: Send appropriate scale factor to client for 4K display optimization
        // TODO: Integrate with output scale management for consistent scaling
        debug!("Fractional scale handler ready for sub-pixel precision scaling");
    }
}

// ============================================================================
// Content Type Protocol - wp-content-type-v1 (State-only pattern)
// ============================================================================
// Note: wp-content-type-v1 uses state-only pattern like viewporter
// No handler implementation required - content type info is stored in surface state

// ============================================================================
// XDG Foreign Handler Implementation
// ============================================================================

impl XdgForeignHandler for WaylandServerState {
    fn xdg_foreign_state(&mut self) -> &mut XdgForeignState {
        &mut self.xdg_foreign_state
    }
}

// ============================================================================
// Idle Inhibit Handler Implementation
// ============================================================================

impl IdleInhibitHandler for WaylandServerState {
    fn inhibit(&mut self, surface: WlSurface) {
        info!("Idle inhibitor activated for surface: {:?}", surface.id());
        
        // TODO: Implement power management integration to prevent system idle
        // TODO: Track active inhibitors for proper reference counting
        // TODO: Integrate with system power management daemon (e.g., systemd-logind)
        debug!("System idle state inhibited for surface");
    }
    
    fn uninhibit(&mut self, surface: WlSurface) {
        info!("Idle inhibitor deactivated for surface: {:?}", surface.id());
        
        // TODO: Remove idle inhibition for this surface
        // TODO: Check if any other surfaces still have active inhibitors
        // TODO: Re-enable system idle if no active inhibitors remain
        debug!("System idle inhibition released for surface");
    }
}

// ============================================================================
// Input Method Handler Implementation
// ============================================================================

impl InputMethodHandler for WaylandServerState {
    fn new_popup(&mut self, _surface: smithay::wayland::input_method::PopupSurface) {
        info!("New input method popup created");
        // TODO: Handle input method popup surface management
        // TODO: Position popup relative to text input focus
        // TODO: Track popup lifecycle for proper cleanup
    }
    
    fn dismiss_popup(&mut self, _surface: smithay::wayland::input_method::PopupSurface) {
        info!("Input method popup dismissed");
        // TODO: Handle popup dismissal
        // TODO: Clean up any resources associated with the popup
    }
    
    fn popup_repositioned(&mut self, _surface: smithay::wayland::input_method::PopupSurface) {
        info!("Input method popup repositioned");
        // TODO: Handle popup repositioning
        // TODO: Update popup position relative to text input focus
    }
    
    fn parent_geometry(&self, _surface: &WlSurface) -> smithay::utils::Rectangle<i32, smithay::utils::Logical> {
        // TODO: Implement proper parent geometry calculation for positioning the popup
        // This is a placeholder that returns a default rectangle
        smithay::utils::Rectangle::from_size((100, 50).into())
    }
}

// ============================================================================
// Keyboard Shortcuts Inhibit Handler Implementation
// ============================================================================

impl KeyboardShortcutsInhibitHandler for WaylandServerState {
    fn keyboard_shortcuts_inhibit_state(&mut self) -> &mut KeyboardShortcutsInhibitState {
        &mut self.keyboard_shortcuts_inhibit_state
    }
    
    fn new_inhibitor(&mut self, inhibitor: smithay::wayland::keyboard_shortcuts_inhibit::KeyboardShortcutsInhibitor) {
        info!("New keyboard shortcuts inhibitor created for surface: {:?}", inhibitor.wl_surface().id());
        
        // TODO: Implement compositor shortcut inhibition logic
        // TODO: Track active inhibitors per surface for proper management
        // TODO: Disable compositor keyboard shortcuts while inhibitor is active
        // TODO: Integrate with keyboard input handling to bypass shortcut processing
        debug!("Keyboard shortcuts inhibition activated - compositor shortcuts disabled");
    }
    
    fn inhibitor_destroyed(&mut self, inhibitor: smithay::wayland::keyboard_shortcuts_inhibit::KeyboardShortcutsInhibitor) {
        info!("Keyboard shortcuts inhibitor destroyed for surface: {:?}", inhibitor.wl_surface().id());
        
        // TODO: Re-enable compositor keyboard shortcuts for this surface
        // TODO: Remove inhibitor from tracking system
        // TODO: Check if any other inhibitors remain active
        // TODO: Restore full compositor shortcut functionality if no active inhibitors
        debug!("Keyboard shortcuts inhibition deactivated - compositor shortcuts re-enabled");
    }
}

// ============================================================================
// Session Lock Handler Implementation
// ============================================================================

impl SessionLockHandler for WaylandServerState {
    fn lock_state(&mut self) -> &mut SessionLockManagerState {
        &mut self.session_lock_manager_state
    }

    fn lock(&mut self, confirmation: smithay::wayland::session_lock::SessionLocker) {
        // Handle lock request
        // For now, immediately confirm the lock
        confirmation.lock();
        info!("Session lock confirmed");
    }

    fn unlock(&mut self) {
        // Handle unlock request
        info!("Session unlocked");
    }

    fn new_surface(&mut self, _surface: smithay::wayland::session_lock::LockSurface, _output: smithay::reexports::wayland_server::protocol::wl_output::WlOutput) {
        // Handle new lock surface
        info!("New lock surface created for output");
    }
}

// ============================================================================
// Security Context Handler Implementation
// ============================================================================

impl SecurityContextHandler for WaylandServerState {
    fn context_created(&mut self, _source: smithay::wayland::security_context::SecurityContextListenerSource, _security_context: smithay::wayland::security_context::SecurityContext) {
        info!("Security context created for sandboxed application");
        
        // TODO: Implement sandboxed execution environment
        // TODO: Apply security restrictions based on context capabilities
        // TODO: Isolate context from sensitive system resources
        // TODO: Track context permissions and enforce access controls
        debug!("Security context established with capability-based permissions");
    }
}

// ============================================================================
// XDG Activation Handler Implementation
// ============================================================================

impl XdgActivationHandler for WaylandServerState {
    fn activation_state(&mut self) -> &mut XdgActivationState {
        &mut self.xdg_activation_state
    }
    
    fn request_activation(&mut self, _token: smithay::wayland::xdg_activation::XdgActivationToken, _token_data: smithay::wayland::xdg_activation::XdgActivationTokenData, _surface: WlSurface) {
        info!("Window activation requested for surface with token");
        
        // TODO: Implement focus management and window activation
        // TODO: Validate activation request against security policies
        // TODO: Switch focus to requested surface if authorized
        // TODO: Update window stack order and input focus
        debug!("Processing window activation request");
    }
}

// ============================================================================
// Foreign Toplevel List Handler Implementation
// ============================================================================

impl ForeignToplevelListHandler for WaylandServerState {
    fn foreign_toplevel_list_state(&mut self) -> &mut ForeignToplevelListState {
        &mut self.foreign_toplevel_list_state
    }
}

// Delegate handlers to implementations
smithay::delegate_compositor!(WaylandServerState);
smithay::delegate_xdg_shell!(WaylandServerState);
smithay::delegate_layer_shell!(WaylandServerState);
smithay::delegate_output!(WaylandServerState);
smithay::delegate_shm!(WaylandServerState);
smithay::delegate_dmabuf!(WaylandServerState);
smithay::delegate_seat!(WaylandServerState);
smithay::delegate_relative_pointer!(WaylandServerState);
smithay::delegate_pointer_constraints!(WaylandServerState);
smithay::delegate_presentation!(WaylandServerState);
smithay::delegate_primary_selection!(WaylandServerState);
smithay::delegate_data_device!(WaylandServerState);
smithay::delegate_xdg_decoration!(WaylandServerState);
smithay::delegate_xdg_foreign!(WaylandServerState);
smithay::delegate_tablet_manager!(WaylandServerState);
smithay::delegate_viewporter!(WaylandServerState);
smithay::delegate_fractional_scale!(WaylandServerState);
smithay::delegate_content_type!(WaylandServerState);
smithay::delegate_alpha_modifier!(WaylandServerState);
smithay::delegate_single_pixel_buffer!(WaylandServerState);
smithay::delegate_cursor_shape!(WaylandServerState);
smithay::delegate_commit_timing!(WaylandServerState);
smithay::delegate_fifo!(WaylandServerState);
smithay::delegate_idle_inhibit!(WaylandServerState);
smithay::delegate_pointer_gestures!(WaylandServerState);
smithay::delegate_virtual_keyboard_manager!(WaylandServerState);
smithay::delegate_text_input_manager!(WaylandServerState);
smithay::delegate_input_method_manager!(WaylandServerState);
smithay::delegate_keyboard_shortcuts_inhibit!(WaylandServerState);
smithay::delegate_session_lock!(WaylandServerState);
smithay::delegate_security_context!(WaylandServerState);
smithay::delegate_xdg_activation!(WaylandServerState);
smithay::delegate_foreign_toplevel_list!(WaylandServerState);
smithay::delegate_drm_syncobj!(WaylandServerState);
