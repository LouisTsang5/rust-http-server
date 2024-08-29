use std::{collections::HashMap, env};

const ENV_ARG_FLAG_PREFIX: &str = "-";

enum ArgType<'a> {
    Flag(&'a str),
    Value(&'a str),
}

impl<'a> ArgType<'a> {
    fn parse(s: &'a str) -> Self {
        if s.starts_with(ENV_ARG_FLAG_PREFIX) {
            ArgType::Flag(&s[1..])
        } else {
            ArgType::Value(s)
        }
    }
}

#[derive(Debug)]
pub struct GetOptError {
    invalid_arg: String,
}

impl std::fmt::Display for GetOptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Invalid argument: {}", self.invalid_arg)
    }
}

impl std::error::Error for GetOptError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }

    fn description(&self) -> &str {
        "description() is deprecated; use Display"
    }

    fn cause(&self) -> Option<&dyn std::error::Error> {
        self.source()
    }
}

pub fn getopt() -> Result<HashMap<String, Option<String>>, GetOptError> {
    let args: Vec<String> = env::args_os()
        .skip(1)
        .map(|arg| arg.to_string_lossy().into())
        .collect();
    let mut args_map: HashMap<String, Option<String>> = HashMap::new();
    let mut cur_flag: Option<&str> = None;
    for arg in &args {
        let arg: ArgType = ArgType::parse(&arg);
        match cur_flag {
            None => match arg {
                ArgType::Flag(f) => cur_flag = Some(f), // Update cur flag
                ArgType::Value(v) => {
                    // Error if value without flag
                    return Err(GetOptError {
                        invalid_arg: v.to_string(),
                    });
                }
            },
            Some(f) => match arg {
                ArgType::Flag(a) => {
                    args_map.insert(f.to_string(), None); // Insert cur_flag if current arg is also a flag
                    cur_flag = Some(a); // Update cur flag as arg
                }
                ArgType::Value(a) => {
                    args_map.insert(f.to_string(), Some(a.to_string())); // Insert cur_flag if current arg is a value
                    cur_flag = None; // Reset cur flag
                }
            },
        }
    }
    if let Some(f) = cur_flag {
        args_map.insert(f.to_string(), None); // Insert cur_flag if last arg is a flag
    }
    Ok(args_map)
}
