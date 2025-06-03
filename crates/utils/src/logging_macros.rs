/// Logging macros for compositor diagnostics
/// 
/// These macros provide convenient ways to log different types of events
/// with automatic context and formatting.

/// Log a startup phase with timing information
#[macro_export]
macro_rules! log_startup_phase {
    ($phase:expr, $($arg:tt)*) => {
        let details = format!($($arg)*);
        tracing::info!("STARTUP[{}]: {}", $phase, details);
        if let Some(logger) = crate::logging::get_logger() {
            let _ = logger.log_startup_phase($phase, &details);
        }
    };
}

/// Log an error with full context
#[macro_export]
macro_rules! log_error_with_context {
    ($error:expr, $context:expr) => {
        tracing::error!("ERROR in {}: {}", $context, $error);
        if let Some(logger) = crate::logging::get_logger() {
            let _ = logger.log_error(&*$error, $context);
        }
    };
}

/// Log hardware/system state changes
#[macro_export]
macro_rules! log_hardware_event {
    ($event:expr, $($arg:tt)*) => {
        let details = format!($($arg)*);
        tracing::info!("HARDWARE[{}]: {}", $event, details);
    };
}

/// Log performance metrics
#[macro_export]
macro_rules! log_performance {
    ($metric:expr, $value:expr, $unit:expr) => {
        tracing::info!("PERF[{}]: {}{}", $metric, $value, $unit);
    };
}

/// Time a block of code and log the duration
#[macro_export]
macro_rules! time_block {
    ($name:expr, $block:block) => {{
        let start = std::time::Instant::now();
        tracing::debug!("TIMING: Starting {}", $name);
        let result = $block;
        let duration = start.elapsed();
        tracing::info!("TIMING: {} completed in {:?}", $name, duration);
        result
    }};
}

/// Log a checkpoint during compositor operation
#[macro_export]
macro_rules! log_checkpoint {
    ($checkpoint:expr, $($arg:tt)*) => {
        let details = format!($($arg)*);
        tracing::info!("CHECKPOINT[{}]: {}", $checkpoint, details);
    };
}
