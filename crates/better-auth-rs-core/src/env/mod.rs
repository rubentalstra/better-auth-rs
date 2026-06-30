//! Interim aggregator for `env/`. The full port of `env/index.ts` (which also re-exports
//! `env-impl` and `color-depth`) lands as those modules are built; for now this wires the logger.
//! Terminal color detection uses std `IsTerminal` rather than porting `color-depth.ts`.

pub mod logger;

pub use logger::{
    InternalLogger, LOGGER, LogHandler, LogLevel, Logger, create_logger, should_publish_log,
};
