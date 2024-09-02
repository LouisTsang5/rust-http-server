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
macro_rules! error {
    ($($arg:tt)*) => {
        eprint!("[{}][ERROR] ", module_path!());
        eprintln!($($arg)*);
    };
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {
        if (crate::log::LOG_LEVEL.get().copied().unwrap() <= crate::log::LogLevel::Warn) {
            print!("[{}][WARN] ", module_path!());
            println!($($arg)*);
        }
    };
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {
        if (crate::log::LOG_LEVEL.get().copied().unwrap() <= crate::log::LogLevel::Info) {
            print!("[{}][INFO] ", module_path!());
            println!($($arg)*);
        }
    };
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {
        if (crate::log::LOG_LEVEL.get().copied().unwrap() <= crate::log::LogLevel::Debug) {
            print!("[{}][DEBUG] ", module_path!());
            println!($($arg)*);
        }
    };
}

#[macro_export]
macro_rules! trace {
    ($($arg:tt)*) => {
        if (crate::log::LOG_LEVEL.get().copied().unwrap() <= crate::log::LogLevel::Trace) {
            print!("[{}][TRACE] ", module_path!());
            println!($($arg)*);
        }
    };
}
