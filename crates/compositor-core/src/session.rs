use libseat::Seat;
use nix::sys::stat::{fchmod, Mode};
use nix::unistd::close;
use std::sync::mpsc;
use std::thread;
use std::collections::HashMap;
use std::path::Path;
use std::os::unix::io::{AsRawFd, RawFd};
use std::os::fd::AsFd;
use compositor_utils::error::{CompositorError, Result};

/// Messages sent to the session thread
#[derive(Debug)]
pub enum SessionMessage {
    /// Request to acquire DRM device access
    AcquireDevice { path: String, response_tx: mpsc::Sender<Result<i32>> },
    /// Request to release DRM device
    ReleaseDevice { fd: i32, response_tx: mpsc::Sender<Result<()>> },
    /// Shutdown the session thread
    Shutdown,
}

/// Messages sent from the session thread
#[derive(Debug)]
pub enum SessionEvent {
    /// Session has been activated (can access devices)
    Activated,
    /// Session has been deactivated (should release devices)
    Deactivated,
    /// Session has been terminated
    Terminated,
}

/// Current state of the session
#[derive(Debug, Clone, PartialEq)]
pub enum SessionState {
    /// Session is inactive (no access to devices)
    Inactive,
    /// Session is active (can access devices)
    Active,
    /// Session is being terminated
    Terminating,
}

/// Session manager for handling DRM device access and privilege separation
pub struct SessionManager {
    /// Channel for sending commands to the session thread
    command_tx: mpsc::Sender<SessionMessage>,
    /// Channel for receiving events from the session thread
    event_rx: mpsc::Receiver<SessionEvent>,
    /// Current session state
    state: SessionState,
    /// Handle to the session thread
    thread_handle: Option<thread::JoinHandle<()>>,
}

impl SessionManager {
    /// Create a new session manager
    pub fn new() -> Result<Self> {
        let (command_tx, command_rx) = mpsc::channel();
        let (event_tx, event_rx) = mpsc::channel();
        
        // Spawn dedicated thread for libseat operations
        let thread_handle = thread::spawn(move || {
            let mut session_thread = SessionThread::new(command_rx, event_tx);
            session_thread.run();
        });
        
        Ok(Self {
            command_tx,
            event_rx,
            state: SessionState::Inactive,
            thread_handle: Some(thread_handle),
        })
    }
    
    /// Acquire access to a DRM device
    pub fn acquire_device(&self, path: String) -> Result<i32> {
        let (response_tx, response_rx) = mpsc::channel();
        
        self.command_tx
            .send(SessionMessage::AcquireDevice { path, response_tx })
            .map_err(|e| CompositorError::system(format!("Failed to send acquire device command: {}", e)))?;
            
        response_rx
            .recv()
            .map_err(|e| CompositorError::system(format!("Failed to receive acquire device response: {}", e)))?
    }
    
    /// Release access to a DRM device
    pub fn release_device(&self, fd: i32) -> Result<()> {
        let (response_tx, response_rx) = mpsc::channel();
        
        self.command_tx
            .send(SessionMessage::ReleaseDevice { fd, response_tx })
            .map_err(|e| CompositorError::system(format!("Failed to send release device command: {}", e)))?;
            
        response_rx
            .recv()
            .map_err(|e| CompositorError::system(format!("Failed to receive release device response: {}", e)))?
    }
    
    /// Check for session events (non-blocking)
    pub fn poll_events(&mut self) -> Vec<SessionEvent> {
        let mut events = Vec::new();
        
        while let Ok(event) = self.event_rx.try_recv() {
            match event {
                SessionEvent::Activated => self.state = SessionState::Active,
                SessionEvent::Deactivated => self.state = SessionState::Inactive,
                SessionEvent::Terminated => self.state = SessionState::Terminating,
            }
            events.push(event);
        }
        
        events
    }

    /// Dispatch libseat events with timeout (for compatibility with backend.rs)
    pub fn dispatch_events(&mut self, timeout_ms: Option<u64>) -> Result<()> {
        // Process any pending events first
        let _events = self.poll_events();
        
        // For now, just sleep for the timeout duration to simulate dispatching
        // In a proper implementation, this would send a message to the session thread
        // to call libseat's dispatch method and wait for the response
        if let Some(ms) = timeout_ms {
            std::thread::sleep(std::time::Duration::from_millis(ms));
        }
        
        Ok(())
    }
    
    /// Get the current session state
    pub fn state(&self) -> SessionState {
        self.state.clone()
    }
    
    /// Check if the session is currently active
    pub fn is_active(&self) -> bool {
        self.state == SessionState::Active
    }

    /// Initialize the session manager (for compatibility with backend.rs)
    pub fn initialize(&mut self) -> Result<()> {
        // Session initialization is done in new(), this is just for compatibility
        Ok(())
    }

    /// Get DRM device file descriptor (for compatibility with backend.rs)
    pub fn get_drm_fd(&self) -> Result<RawFd> {
        // For now, return a placeholder - in a real implementation this would
        // track opened devices and return the appropriate FD
        Err(CompositorError::system("DRM device not yet implemented".to_string()))
    }
}

impl Drop for SessionManager {
    fn drop(&mut self) {
        // Send shutdown command
        let _ = self.command_tx.send(SessionMessage::Shutdown);
        
        // Wait for thread to finish
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }
}

/// Session thread that handles libseat operations
struct SessionThread {
    command_rx: mpsc::Receiver<SessionMessage>,
    event_tx: mpsc::Sender<SessionEvent>,
    seat: Option<Seat>,
    device_fds: HashMap<String, i32>,
}

impl SessionThread {
    fn new(command_rx: mpsc::Receiver<SessionMessage>, event_tx: mpsc::Sender<SessionEvent>) -> Self {
        Self {
            command_rx,
            event_tx,
            seat: None,
            device_fds: HashMap::new(),
        }
    }
    
    fn run(&mut self) {
        // Initialize libseat session
        if let Err(e) = self.initialize_seat() {
            eprintln!("Failed to initialize seat: {}", e);
            return;
        }
        
        // Main event loop
        while let Ok(message) = self.command_rx.recv() {
            match message {
                SessionMessage::AcquireDevice { path, response_tx } => {
                    let result = self.handle_acquire_device(&path);
                    let _ = response_tx.send(result);
                }
                SessionMessage::ReleaseDevice { fd, response_tx } => {
                    let result = self.handle_release_device(fd);
                    let _ = response_tx.send(result);
                }
                SessionMessage::Shutdown => break,
            }
        }
        
        // Cleanup
        self.cleanup();
    }
    
    fn initialize_seat(&mut self) -> Result<()> {
        // Create a simple callback that sends events to our channel
        let event_tx = self.event_tx.clone();
        
        let callback = move |_seat: &mut libseat::SeatRef, seat_event: libseat::SeatEvent| {
            match seat_event {
                libseat::SeatEvent::Enable => {
                    let _ = event_tx.send(SessionEvent::Activated);
                }
                libseat::SeatEvent::Disable => {
                    let _ = event_tx.send(SessionEvent::Deactivated);
                }
            }
        };
        
        // Try to open a libseat session
        let seat = Seat::open(callback)
            .map_err(|e| CompositorError::system(format!("Failed to open libseat session: {}", e)))?;
            
        self.seat = Some(seat);
        
        // Send initial activation event for testing
        let _ = self.event_tx.send(SessionEvent::Activated);
        
        Ok(())
    }
    
    fn handle_acquire_device(&mut self, path: &str) -> Result<i32> {
        let seat = self.seat.as_mut()
            .ok_or_else(|| CompositorError::system("Seat not initialized".to_string()))?;
            
        let device_path = Path::new(path);
        let seat_device = seat.open_device(&device_path)
            .map_err(|e| CompositorError::system(format!("Failed to open device {}: {}", path, e)))?;
            
        let fd = seat_device.as_fd().as_raw_fd();
        
        // Set proper permissions on the device
        let mode = Mode::S_IRUSR | Mode::S_IWUSR | Mode::S_IRGRP | Mode::S_IWGRP;
        if let Err(e) = fchmod(fd, mode) {
            eprintln!("Warning: Failed to set device permissions: {}", e);
        }
        
        self.device_fds.insert(path.to_string(), fd);
        
        // Keep the seat_device alive by storing it
        // For now, we'll forget it to prevent it from closing when dropped
        // This is not ideal but works for basic functionality
        std::mem::forget(seat_device);
        
        Ok(fd)
    }
    
    fn handle_release_device(&mut self, fd: i32) -> Result<()> {
        // For now, just remove from tracking
        // In a proper implementation, we'd use libseat's close_device
        self.device_fds.retain(|_, &mut dev_fd| dev_fd != fd);
        
        // Close the file descriptor manually since we used mem::forget earlier
        let _ = close(fd);
        
        Ok(())
    }
    
    fn cleanup(&mut self) {
        // Close all open devices manually
        for &fd in self.device_fds.values() {
            let _ = close(fd);
        }
        
        self.device_fds.clear();
        self.seat = None;
    }
}
