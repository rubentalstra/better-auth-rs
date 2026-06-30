//! Behavior tests for the logger.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::{Arc, Mutex};

use super::*;

#[test]
fn should_publish_respects_levels() {
    // threshold Warn: warn/error publish; debug/info/success do not.
    assert!(should_publish_log(LogLevel::Warn, LogLevel::Warn));
    assert!(should_publish_log(LogLevel::Warn, LogLevel::Error));
    assert!(!should_publish_log(LogLevel::Warn, LogLevel::Info));
    assert!(!should_publish_log(LogLevel::Warn, LogLevel::Debug));
    assert!(!should_publish_log(LogLevel::Warn, LogLevel::Success));
    // success sits between info and warn.
    assert!(should_publish_log(LogLevel::Info, LogLevel::Success));
}

type Calls = Arc<Mutex<Vec<(LogLevel, String)>>>;

fn capturing_handler() -> (LogHandler, Calls) {
    let calls: Calls = Arc::new(Mutex::new(Vec::new()));
    let captured = calls.clone();
    let handler: LogHandler =
        Arc::new(move |level, msg| captured.lock().unwrap().push((level, msg.to_owned())));
    (handler, calls)
}

#[test]
fn handler_receives_published_messages_and_maps_success_to_info() {
    let (handler, calls) = capturing_handler();
    let log = create_logger(Some(Logger {
        level: Some(LogLevel::Debug),
        log: Some(handler),
        ..Default::default()
    }));
    log.warn("w");
    log.success("s"); // success -> info for the handler
    log.debug("d");
    assert_eq!(
        *calls.lock().unwrap(),
        vec![
            (LogLevel::Warn, "w".to_owned()),
            (LogLevel::Info, "s".to_owned()),
            (LogLevel::Debug, "d".to_owned()),
        ]
    );
}

#[test]
fn below_threshold_is_not_published() {
    let (handler, calls) = capturing_handler();
    let log = create_logger(Some(Logger {
        level: Some(LogLevel::Warn),
        log: Some(handler),
        ..Default::default()
    }));
    log.info("ignored");
    log.error("kept");
    assert_eq!(
        *calls.lock().unwrap(),
        vec![(LogLevel::Error, "kept".to_owned())]
    );
}

#[test]
fn disabled_logger_emits_nothing() {
    let (handler, calls) = capturing_handler();
    let log = create_logger(Some(Logger {
        disabled: Some(true),
        level: Some(LogLevel::Debug),
        log: Some(handler),
        ..Default::default()
    }));
    log.error("nope");
    assert!(calls.lock().unwrap().is_empty());
}

#[test]
fn level_getter_defaults_to_warn() {
    assert_eq!(create_logger(None).level(), LogLevel::Warn);
    assert_eq!(
        create_logger(Some(Logger {
            level: Some(LogLevel::Error),
            ..Default::default()
        }))
        .level(),
        LogLevel::Error
    );
}
