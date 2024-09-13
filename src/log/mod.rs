use std::fmt::{Display, Formatter};

use tokio::sync::{OnceCell, SetError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Error = 4,
    Warn = 3,
    Info = 2,
    Debug = 1,
    Trace = 0,
}

impl Default for LogLevel {
    fn default() -> Self {
        crate::DEFAULT_LOG_LEVEL
    }
}

impl Display for LogLevel {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            LogLevel::Error => "Error",
            LogLevel::Warn => "Warn",
            LogLevel::Info => "Info",
            LogLevel::Debug => "Debug",
            LogLevel::Trace => "Trace",
        };
        write!(f, "{}", s)
    }
}

impl From<&str> for LogLevel {
    fn from(s: &str) -> Self {
        let s = s.to_lowercase();
        match &s as &str {
            "error" => LogLevel::Error,
            "warn" => LogLevel::Warn,
            "info" => LogLevel::Info,
            "debug" => LogLevel::Debug,
            "trace" => LogLevel::Trace,
            _ => Self::default(),
        }
    }
}

impl From<&String> for LogLevel {
    fn from(s: &String) -> Self {
        LogLevel::from(s.as_str())
    }
}

pub static LOG_LEVEL: OnceCell<LogLevel> = OnceCell::const_new();
pub fn set_log_level(level: LogLevel) -> Result<(), SetError<LogLevel>> {
    LOG_LEVEL.set(level)
}
pub fn get_log_level() -> LogLevel {
    LOG_LEVEL.get().copied().unwrap()
}

#[macro_export]
macro_rules! log_ctx {
    ($ctx:expr) => {
        const _LOG_CTX_JK23BN4KJ2: &str = $ctx;
    };
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {
        {
            eprint!("[{}][ERROR] ", _LOG_CTX_JK23BN4KJ2);
            eprintln!($($arg)*);
        }
    };
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {
        if (crate::log::LOG_LEVEL.get().copied().unwrap() <= crate::log::LogLevel::Warn) {
            print!("[{}][WARN] ", _LOG_CTX_JK23BN4KJ2);
            println!($($arg)*);
        }
    };
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {
        if (crate::log::LOG_LEVEL.get().copied().unwrap() <= crate::log::LogLevel::Info) {
            print!("[{}][INFO] ", _LOG_CTX_JK23BN4KJ2);
            println!($($arg)*);
        }
    };
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {
        if (crate::log::LOG_LEVEL.get().copied().unwrap() <= crate::log::LogLevel::Debug) {
            print!("[{}][DEBUG] ", _LOG_CTX_JK23BN4KJ2);
            println!($($arg)*);
        }
    };
}

#[macro_export]
macro_rules! trace {
    ($($arg:tt)*) => {
        if (crate::log::LOG_LEVEL.get().copied().unwrap() <= crate::log::LogLevel::Trace) {
            print!("[{}][TRACE] ", _LOG_CTX_JK23BN4KJ2);
            println!($($arg)*);
        }
    };
}

#[macro_export]
macro_rules! timer {
    ($ctx:expr) => {
        let _timer_jk23_bn4_kj2 =
            if crate::log::LOG_LEVEL.get().copied().unwrap() <= crate::log::LogLevel::Debug {
                Some(crate::log::Timer::new(_LOG_CTX_JK23BN4KJ2, $ctx))
            } else {
                None
            };
    };
}

pub struct Timer<'a> {
    start: std::time::Instant,
    log_ctx: &'a str,
    timer_ctx: &'a str,
}

impl<'a> Timer<'a> {
    pub fn new(log_ctx: &'a str, timer_ctx: &'a str) -> Self {
        Self {
            start: std::time::Instant::now(),
            log_ctx,
            timer_ctx,
        }
    }
}

impl Drop for Timer<'_> {
    fn drop(&mut self) {
        let elapsed = self.start.elapsed();
        println!(
            "[{}][DEBUG][{}] elipsed {}μs",
            self.log_ctx,
            self.timer_ctx,
            elapsed.as_micros()
        );
    }
}
