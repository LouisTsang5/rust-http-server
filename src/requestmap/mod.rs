use std::{
    borrow::Cow,
    collections::HashMap,
    error::Error,
    fmt::Display,
    num::ParseIntError,
    path::{Path, PathBuf},
};

use rand::{thread_rng, Rng};

// Define delimiters
// Sample of single map entry: /path=path/to/file.txt
// Sample of weighted map entry: /path=path/to/file1.txt'10,path/to/file2.txt'20
const REQ_MAP_KEY_VAL_DELIM: char = '=';
const REQ_MAP_VAL_DELIM: char = ',';
const REQ_MAP_VAL_WEIGHT_DELIM: char = '\'';
const STRING_INIT_SIZE: usize = 64;

#[derive(Debug)]
struct RandPath {
    path: PathBuf,
    weight: u32,
}

#[derive(Debug)]
enum PathEntry {
    Single(PathBuf),
    Weighted(Vec<RandPath>),
}

#[derive(Debug)]
pub struct RequestMap {
    map: HashMap<String, PathEntry>,
}

#[derive(Debug, Clone)]
enum ErrorKind {
    MissingDelim(char),
    InvalidWeight(ParseIntError),
    InvalidPath,
    InvalidKey,
}

#[derive(Debug, Clone)]
pub struct RequestMapParseError {
    line_num: usize,
    kind: ErrorKind,
}

impl Display for RequestMapParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Failed to parse request map. {} at line {}",
            match &self.kind {
                ErrorKind::MissingDelim(c) => Cow::Owned(format!("Missing delimiter {}", c)),
                ErrorKind::InvalidWeight(e) => Cow::Owned(format!("Invalid weight ({})", e)),
                ErrorKind::InvalidPath => Cow::Borrowed("Invalid path"),
                ErrorKind::InvalidKey => Cow::Borrowed("Invalid key"),
            },
            self.line_num
        )
    }
}

impl Error for RequestMapParseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }

    fn description(&self) -> &str {
        "description() is deprecated; use Display"
    }

    fn cause(&self) -> Option<&dyn Error> {
        self.source()
    }
}

impl RequestMap {
    pub fn parse_str(map_str: &str) -> Result<Self, RequestMapParseError> {
        // Construct the map
        let mut request_map = HashMap::new();
        for (line_num, line) in map_str.lines().enumerate() {
            let line_num = line_num + 1;

            // Split into key and value
            let (k, v) = line
                .split_once(REQ_MAP_KEY_VAL_DELIM)
                .ok_or(RequestMapParseError {
                    line_num,
                    kind: ErrorKind::MissingDelim(REQ_MAP_KEY_VAL_DELIM),
                })?;
            let k = k.trim();
            let v = v.trim();

            // Return err if v is empty
            if v.is_empty() {
                return Err(RequestMapParseError {
                    line_num,
                    kind: ErrorKind::InvalidPath,
                });
            }

            // Return err if k is empty
            if k.is_empty() {
                return Err(RequestMapParseError {
                    line_num,
                    kind: ErrorKind::InvalidKey,
                });
            }

            // Split value into paths
            let v = v.split(REQ_MAP_VAL_DELIM).collect::<Vec<&str>>();
            if v.len() > 1 {
                // Weighted path
                let mut weighted_paths = Vec::new();
                for entry in v {
                    // Split into path and weight
                    let (path, weight) =
                        entry
                            .split_once(REQ_MAP_VAL_WEIGHT_DELIM)
                            .ok_or(RequestMapParseError {
                                line_num,
                                kind: ErrorKind::MissingDelim(REQ_MAP_VAL_WEIGHT_DELIM),
                            })?;
                    let path = path.trim();
                    let weight = weight.trim();

                    // Parse weight
                    let weight = weight.parse::<u32>().map_err(|e| RequestMapParseError {
                        line_num,
                        kind: ErrorKind::InvalidWeight(e),
                    })?;

                    // Add to weighted paths
                    weighted_paths.push(RandPath {
                        path: PathBuf::from(path),
                        weight,
                    });
                }
                request_map.insert(k.to_string(), PathEntry::Weighted(weighted_paths));
            } else {
                // Single path
                request_map.insert(k.to_string(), PathEntry::Single(PathBuf::from(v[0])));
            }
        }

        Ok(Self { map: request_map })
    }

    pub fn get(&self, k: &str) -> Option<&Path> {
        self.map.get(k).map(|p| match p {
            // Return path directly if it is single
            PathEntry::Single(p) => p.as_path(),

            // Choose a random path based on weight
            PathEntry::Weighted(p) => {
                // Calculate total weight
                let total_weight = p
                    .iter()
                    .map(|p| p.weight)
                    .reduce(|acc, cur| acc + cur)
                    .unwrap();

                // Generate a random number
                let mut rand_num = thread_rng().gen_range(0..total_weight);

                // Choose a path based on random number
                for rp in p {
                    if rand_num < rp.weight {
                        return rp.path.as_path();
                    }
                    rand_num -= rp.weight;
                }
                panic!("Random number out of range");
            }
        })
    }
}

impl Display for RequestMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (k, v) in &self.map {
            match v {
                PathEntry::Single(p) => write!(f, "{} -> {}\n", k, p.display())?,
                PathEntry::Weighted(p) => {
                    let mut line = String::with_capacity(STRING_INIT_SIZE);
                    line.push_str(&format!("{} -> ", k));
                    for rp in p {
                        line.push_str(&format!("{}'{} ", rp.path.display(), rp.weight));
                    }
                    write!(f, "{}\n", line)?;
                }
            }
        }
        Ok(())
    }
}
