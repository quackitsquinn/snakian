use core::{cell::UnsafeCell, mem::MaybeUninit};

use conquer_once::spin::OnceCell;
use log::Log;
use spin::Mutex;

use crate::{display, serial_println, prelude::*};




pub struct LogHandler {
    has_display_init: bool,
    display_level: log::Level,
    level: log::Level,
}

impl Log for LogHandler {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            if self.has_display_init && record.level() <= self.display_level {
                println!("[{}:{}]: {} - {}", record.file().unwrap_or("unknown"), record.line().unwrap_or(0), record.level(), record.args());
            } else {
                // If the display has not been initialized, print to the serial port.
                // This is in an else block because the `println!` macro prints to both the display and the serial port.
                serial_println!("[{}:{}]: {} - {}", record.file().unwrap_or("unknown"), record.line().unwrap_or(0), record.level(), record.args());
            }
        }
    }

    fn flush(&self) {
        // We have no buffer to flush.
    }
}

impl LogHandler {
    pub const fn new(level: log::Level, display_level: log::Level) -> Self {
        Self {
            has_display_init: false,
            display_level,
            level,
        }
    }

    pub fn init_display(&mut self) {
        self.has_display_init = true;
    }
}
/// This is a wrapper around the `LogHandler` struct that allows it to be used as a global static.
/// This is necessary because the `log` crate requires a static reference to a logger.
pub struct LoggerInstance {
    /// The inner `LogHandler` is wrapped in an `UnsafeCell` to allow for interior mutability.
    inner: UnsafeCell<LogHandler>,
    /// A lock to ensure that multiple threads do not attempt to initialize the logger at the same time.
    lock: Mutex<()>,
}

unsafe impl Sync for LoggerInstance {}


impl LoggerInstance {
    /// Creates a new `LoggerInstance` with the default log level and display level.
    const fn new() -> Self {
        Self {
            inner: UnsafeCell::new(LogHandler::new(log::Level::Trace, log::Level::Trace)),
            lock: Mutex::new(()),
        }
    }
    /// Initializes the logger with the given log level and display level.
    pub fn init(&self, level: log::Level, display_level: log::Level) {
        let ctx = self.lock.lock();
        let logger = self.inner.get();
        unsafe {
            let logger = logger.as_mut().unwrap();
            logger.level = level;
            logger.display_level = display_level;
        }
        drop(ctx);
    }
    /// Initializes the logger with the given log level and display level.
    pub fn init_display(&self) {
        let ctx = self.lock.lock();
        let logger = self.inner.get();
        unsafe {
            let logger = logger.as_mut().unwrap();
            logger.init_display();
        }
        drop(ctx);
    }
    /// Gets a reference to the logger. This function is unsafe because it returns a reference to an `UnsafeCell` contents.
    pub unsafe fn get_logger(&self) -> &LogHandler {
        unsafe { &*self.inner.get() }
    }
}

static LOGGER: LoggerInstance = LoggerInstance::new();

pub fn init_logger(level: log::Level, display_level: log::Level) {
    LOGGER.init(level, display_level);
    log::set_logger(unsafe { LOGGER.inner.get().as_mut().unwrap()}).unwrap();
    log::set_max_level(level.to_level_filter());
}

pub fn init_display_logger() {
    LOGGER.init_display();
}