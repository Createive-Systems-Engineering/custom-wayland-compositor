use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};
use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::Write;
use chrono::{DateTime, Local};
use serde_json::json;
use std::sync::{Arc, Mutex};
use once_cell::sync::OnceCell;

/// Global logger instance
static GLOBAL_LOGGER: OnceCell<Arc<Mutex<CompositorLogger>>> = OnceCell::new();

/// Comprehensive logging system for compositor diagnostics
pub struct CompositorLogger {
    log_dir: PathBuf,
    session_id: String,
    startup_log: File,
    error_log: File,
    hardware_log: File,
}

impl CompositorLogger {
    pub fn new() -> anyhow::Result<Self> {
        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        let session_id = format!("compositor_session_{}", timestamp);
        
        let log_dir = std::env::var("COMPOSITOR_LOG_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/tmp/custom_compositor_logs"));
        
        // Create session-specific log directory
        let session_log_dir = log_dir.join(&session_id);
        std::fs::create_dir_all(&session_log_dir)?;
        
        // Create specialized log files
        let startup_log = File::create(session_log_dir.join("startup.log"))?;
        let error_log = File::create(session_log_dir.join("errors.log"))?;
        let hardware_log = File::create(session_log_dir.join("hardware.log"))?;
        
        let logger = Self {
            log_dir: session_log_dir,
            session_id,
            startup_log,
            error_log,
            hardware_log,
        };
        
        // Log initial system state
        logger.log_system_state()?;
        
        Ok(logger)
    }
    
    /// Log comprehensive system state for debugging
    pub fn log_system_state(&self) -> anyhow::Result<()> {
        let mut hardware_log = &self.hardware_log;
        
        writeln!(hardware_log, "=== COMPOSITOR SESSION: {} ===", self.session_id)?;
        writeln!(hardware_log, "Timestamp: {}", Local::now().to_rfc3339())?;
        writeln!(hardware_log)?;
        
        // Log environment variables
        writeln!(hardware_log, "=== ENVIRONMENT ===")?;
        for (key, value) in std::env::vars() {
            if key.contains("DISPLAY") || key.contains("WAYLAND") || key.contains("XDG") || 
               key.contains("QT") || key.contains("GDK") || key.contains("RUNTIME") {
                writeln!(hardware_log, "{}: {}", key, value)?;
            }
        }
        writeln!(hardware_log)?;
        
        // Log DRM devices
        writeln!(hardware_log, "=== DRM DEVICES ===")?;
        if let Ok(entries) = std::fs::read_dir("/dev/dri") {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    let accessible = std::fs::OpenOptions::new()
                        .read(true)
                        .write(true)
                        .open(&path)
                        .is_ok();
                    writeln!(hardware_log, "{}: accessible={}", name, accessible)?;
                }
            }
        }
        writeln!(hardware_log)?;
        
        // Log current displays (if available)
        writeln!(hardware_log, "=== DISPLAY INFO ===")?;
        if let Ok(output) = std::process::Command::new("xrandr").output() {
            writeln!(hardware_log, "xrandr output:")?;
            writeln!(hardware_log, "{}", String::from_utf8_lossy(&output.stdout))?;
        }
        writeln!(hardware_log)?;
        
        // Log system info
        writeln!(hardware_log, "=== SYSTEM INFO ===")?;
        if let Ok(output) = std::process::Command::new("uname").arg("-a").output() {
            writeln!(hardware_log, "Kernel: {}", String::from_utf8_lossy(&output.stdout).trim())?;
        }
        
        if let Ok(contents) = std::fs::read_to_string("/proc/meminfo") {
            if let Some(line) = contents.lines().find(|l| l.starts_with("MemTotal:")) {
                writeln!(hardware_log, "{}", line)?;
            }
        }
        
        hardware_log.flush()?;
        Ok(())
    }
    
    /// Log startup phase with detailed timing
    pub fn log_startup_phase(&mut self, phase: &str, details: &str) -> anyhow::Result<()> {
        let timestamp = Local::now().to_rfc3339();
        writeln!(self.startup_log, "[{}] PHASE: {} - {}", timestamp, phase, details)?;
        self.startup_log.flush()?;
        Ok(())
    }
    
    /// Log errors with full context and stack traces
    pub fn log_error(&mut self, error_msg: &str, context: &str) -> anyhow::Result<()> {
        let timestamp = Local::now().to_rfc3339();
        
        writeln!(self.error_log, "[{}] ERROR in {}", timestamp, context)?;
        writeln!(self.error_log, "Error: {}", error_msg)?;
        writeln!(self.error_log, "---")?;
        self.error_log.flush()?;
        Ok(())
    }
    
    /// Get log directory path
    pub fn log_dir(&self) -> &Path {
        &self.log_dir
    }
    
    /// Get session ID
    pub fn session_id(&self) -> &str {
        &self.session_id
    }
}

/// Initialize the enhanced logging system for the compositor
pub fn setup_logging() -> anyhow::Result<()> {
    // Create the enhanced logger first
    let logger = CompositorLogger::new()?;
    let session_id = logger.session_id().to_string();
    let log_dir = logger.log_dir().to_path_buf();
    
    // Store logger globally
    GLOBAL_LOGGER.set(Arc::new(Mutex::new(logger)))
        .map_err(|_| anyhow::anyhow!("Failed to set global logger"))?;
    
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,custom_compositor=debug,compositor_core=debug,vulkan_renderer=debug"));

    // Console layer with pretty formatting
    let console_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_thread_ids(true)
        .with_level(true)
        .with_ansi(true);

    // File appender for main log
    let file_appender = tracing_appender::rolling::never(&log_dir, "main.log");
    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(file_appender)
        .with_ansi(false)
        .json();

    // Error file layer for errors only
    let error_file_appender = tracing_appender::rolling::never(&log_dir, "tracing_errors.log");
    let error_file_layer = tracing_subscriber::fmt::layer()
        .with_writer(error_file_appender)
        .with_ansi(false)
        .with_filter(EnvFilter::new("error"));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(console_layer)
        .with(file_layer)
        .with(error_file_layer)
        .init();

    tracing::info!("Enhanced logging system initialized");
    tracing::info!("Session ID: {}", session_id);
    tracing::info!("Log directory: {}", log_dir.display());
    
    Ok(())
}

/// Get global logger instance
pub fn get_logger() -> Option<Arc<Mutex<CompositorLogger>>> {
    GLOBAL_LOGGER.get().cloned()
}

/// Log startup phase using global logger
pub fn log_startup_phase(phase: &str, details: &str) {
    if let Some(logger_arc) = get_logger() {
        if let Ok(mut logger) = logger_arc.lock() {
            let _ = logger.log_startup_phase(phase, details);
        }
    }
    tracing::info!("STARTUP[{}]: {}", phase, details);
}

/// Log error using global logger
pub fn log_error_with_context(error_msg: &str, context: &str) {
    if let Some(logger_arc) = get_logger() {
        if let Ok(mut logger) = logger_arc.lock() {
            let _ = logger.log_error(error_msg, context);
        }
    }
    tracing::error!("ERROR in {}: {}", context, error_msg);
}

/// Setup logging for testing - simplified output
pub fn setup_test_logging() {
    let _ = tracing_subscriber::fmt()
        .with_test_writer()
        .with_env_filter("debug")
        .try_init();
}
