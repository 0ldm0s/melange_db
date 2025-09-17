//! 高性能日志模块
//!
//! 使用tracing替代标准log库，提供零成本抽象和更好的性能

/// 调试级别日志 - 仅在debug模式下编译
#[macro_export]
macro_rules! debug_log {
    ($($arg:tt)*) => {
        #[cfg(debug_assertions)]
        tracing::debug!($($arg)*);

        #[cfg(not(debug_assertions))]
        {
            // release模式下完全零成本
        }
    };
}

/// 追踪级别日志 - 仅在debug模式下编译
#[macro_export]
macro_rules! trace_log {
    ($($arg:tt)*) => {
        #[cfg(debug_assertions)]
        tracing::trace!($($arg)*);
    };
}

/// 信息级别日志 - 轻量级，仅在必要时使用
#[macro_export]
macro_rules! info_log {
    ($($arg:tt)*) => {
        #[cfg(debug_assertions)]
        tracing::info!($($arg)*);
    };
}

/// 警告级别日志 - 始终保留但优化
#[macro_export]
macro_rules! warn_log {
    ($($arg:tt)*) => {
        tracing::warn!($($arg)*);
    };
}

/// 错误级别日志 - 始终保留
#[macro_export]
macro_rules! error_log {
    ($($arg:tt)*) => {
        tracing::error!($($arg)*);
    };
}

/// 初始化日志系统
pub fn init_logging() {
    use tracing_subscriber::{fmt, EnvFilter};

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_thread_ids(false)
        .with_thread_names(false)
        .init();
}

/// 性能关键路径的零成本日志
/// 使用条件编译确保在release模式下完全无开销
#[macro_export]
macro_rules! perf_trace {
    ($($arg:tt)*) => {
        #[cfg(all(debug_assertions, feature = "perf-trace"))]
        {
            use std::time::Instant;
            static _COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let start = Instant::now();
            let result = { $($arg)* };
            let elapsed = start.elapsed();
            let count = _COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if count % 1000 == 0 {
                debug_log!("perf_trace: operation took {:?}", elapsed);
            }
            result
        }
        #[cfg(not(all(debug_assertions, feature = "perf-trace")))]
        { $($arg)* }
    };
}