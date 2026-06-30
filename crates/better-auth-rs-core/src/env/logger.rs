//! Upstream reference: env/logger.ts
//!
//! A configurable console logger: levels, optional ANSI colors, and an optional custom handler
//! (`Logger.log`). Color detection uses std [`IsTerminal`] on stderr rather than porting
//! `env/color-depth.ts`'s Node color-depth heuristics. The custom handler's variadic console args
//! are dropped (message only).

use std::io::{IsTerminal, Write};
use std::sync::{Arc, LazyLock};

use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

const RESET: &str = "\x1b[0m";
const DIM: &str = "\x1b[2m";
const BRIGHT: &str = "\x1b[1m";

/// Log levels, in increasing order of the verbosity threshold (`LogLevel`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    /// Verbose diagnostics.
    Debug,
    /// Informational messages.
    Info,
    /// Success messages (treated as `Info` for custom handlers).
    Success,
    /// Warnings.
    Warn,
    /// Errors.
    Error,
}

impl LogLevel {
    const fn index(self) -> usize {
        match self {
            LogLevel::Debug => 0,
            LogLevel::Info => 1,
            LogLevel::Success => 2,
            LogLevel::Warn => 3,
            LogLevel::Error => 4,
        }
    }

    /// The uppercase label shown in output.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Success => "SUCCESS",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
        }
    }

    const fn color(self) -> &'static str {
        match self {
            LogLevel::Debug => "\x1b[35m",   // magenta
            LogLevel::Info => "\x1b[34m",    // blue
            LogLevel::Success => "\x1b[32m", // green
            LogLevel::Warn => "\x1b[33m",    // yellow
            LogLevel::Error => "\x1b[31m",   // red
        }
    }
}

/// Whether a message at `level` should be published given the configured `current` threshold.
#[must_use]
pub fn should_publish_log(current: LogLevel, level: LogLevel) -> bool {
    level.index() >= current.index()
}

/// A custom log handler (`Logger.log`). The level passed is never `Success` (it is mapped to
/// `Info`), mirroring upstream `Exclude<LogLevel, "success">`.
pub type LogHandler = Arc<dyn Fn(LogLevel, &str) + Send + Sync>;

/// Logger configuration (`Logger`) — what a user passes via `BetterAuthOptions.logger`.
#[derive(Clone, Default)]
pub struct Logger {
    /// Disable all logging.
    pub disabled: Option<bool>,
    /// Force colors off (`true`) or on (`false`); auto-detected from the terminal when `None`.
    pub disable_colors: Option<bool>,
    /// The minimum level to publish (defaults to `Warn`). `Success` is not a valid threshold.
    pub level: Option<LogLevel>,
    /// A custom handler; when set, it receives messages instead of the console.
    pub log: Option<LogHandler>,
}

impl core::fmt::Debug for Logger {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Logger")
            .field("disabled", &self.disabled)
            .field("disable_colors", &self.disable_colors)
            .field("level", &self.level)
            .field("log", &self.log.as_ref().map(|_| "<fn>"))
            .finish()
    }
}

fn format_message(level: LogLevel, message: &str, colors: bool) -> String {
    let ts = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_default();
    if colors {
        format!(
            "{DIM}{ts}{RESET} {}{}{RESET} {BRIGHT}[Better Auth]:{RESET} {message}",
            level.color(),
            level.label()
        )
    } else {
        format!("{ts} {} [Better Auth]: {message}", level.label())
    }
}

/// A built logger (`InternalLogger`) with one method per level.
pub struct InternalLogger {
    enabled: bool,
    log_level: LogLevel,
    colors: bool,
    handler: Option<LogHandler>,
}

impl core::fmt::Debug for InternalLogger {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("InternalLogger")
            .field("enabled", &self.enabled)
            .field("log_level", &self.log_level)
            .field("colors", &self.colors)
            .field("handler", &self.handler.as_ref().map(|_| "<fn>"))
            .finish()
    }
}

impl InternalLogger {
    /// Log at `Debug`.
    pub fn debug(&self, message: &str) {
        self.emit(LogLevel::Debug, message);
    }
    /// Log at `Info`.
    pub fn info(&self, message: &str) {
        self.emit(LogLevel::Info, message);
    }
    /// Log at `Success`.
    pub fn success(&self, message: &str) {
        self.emit(LogLevel::Success, message);
    }
    /// Log at `Warn`.
    pub fn warn(&self, message: &str) {
        self.emit(LogLevel::Warn, message);
    }
    /// Log at `Error`.
    pub fn error(&self, message: &str) {
        self.emit(LogLevel::Error, message);
    }
    /// The configured threshold level.
    #[must_use]
    pub fn level(&self) -> LogLevel {
        self.log_level
    }

    fn emit(&self, level: LogLevel, message: &str) {
        if !self.enabled || !should_publish_log(self.log_level, level) {
            return;
        }
        if let Some(handler) = &self.handler {
            let mapped = if level == LogLevel::Success {
                LogLevel::Info
            } else {
                level
            };
            handler(mapped, message);
            return;
        }
        let formatted = format_message(level, message, self.colors);
        match level {
            LogLevel::Error | LogLevel::Warn => {
                let _ = writeln!(std::io::stderr(), "{formatted}");
            }
            _ => {
                let _ = writeln!(std::io::stdout(), "{formatted}");
            }
        }
    }
}

/// Build a logger from optional configuration (`createLogger`).
#[must_use]
pub fn create_logger(options: Option<Logger>) -> InternalLogger {
    let options = options.unwrap_or_default();
    let enabled = options.disabled != Some(true);
    let log_level = options.level.unwrap_or(LogLevel::Warn);
    let colors = match options.disable_colors {
        Some(disable) => !disable,
        None => std::io::stderr().is_terminal(),
    };
    InternalLogger {
        enabled,
        log_level,
        colors,
        handler: options.log,
    }
}

/// The default logger instance (`logger` in upstream).
pub static LOGGER: LazyLock<InternalLogger> = LazyLock::new(|| create_logger(None));

#[cfg(test)]
#[path = "logger.test.rs"]
mod logger_tests;
