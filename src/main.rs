mod filecache;
mod fswatcher;
mod getopt;
mod http;
mod log;
mod requestmap;
mod teewriter;
mod util;

use filecache::FileCache;
use fswatcher::setup_fs_watcher;
use getopt::getopt;
use http::handle_connection;
use log::LogLevel;
use requestmap::RequestMap;
use std::{env, path::PathBuf, sync::Arc};
use tokio::{
    fs::read_to_string,
    net::TcpListener,
    select,
    task::{self},
};
use util::fmt_size;

// Constants
const BUFF_INIT_SIZE: usize = 1024; // Referencial init buffer size of all program buffers. All buffers are initialized using multiples of this value.
const DEFAULT_PORT: u16 = 3006;
const DEFAULT_FILE_CACHE_SIZE: usize = 100 * 1024 * 1024;
const DEFAULT_LOG_LEVEL: LogLevel = LogLevel::Info;
const RES_ROOT_FOLDER: &str = "res";
const REQ_MAP_FILE: &str = "map.txt";
const ENV_ARG_PORT_KEY: &str = "p";
const ENV_ARG_FILE_ROOT_KEY: &str = "f";
const ENV_ARG_FILE_CACHE_SIZE_KEY: &str = "c";
const ENV_ARG_LOG_LEVEL_KEY: &str = "l";
log_ctx!("Main");

struct Config {
    file_root: PathBuf,
    port: u16,
    file_cache_size: usize,
    log_level: LogLevel,
}

fn get_config() -> Result<Config, Box<dyn std::error::Error>> {
    let args = getopt()?;

    // get port
    let port = match args.get(ENV_ARG_PORT_KEY) {
        Some(p) => match p {
            Some(p) => match p.parse::<u16>() {
                Ok(p) => p,
                Err(e) => return Err(format!("Invalid port: {}", e).into()),
            },
            None => DEFAULT_PORT,
        },
        None => DEFAULT_PORT,
    };

    // get file root
    let file_root = match args.get(ENV_ARG_FILE_ROOT_KEY) {
        Some(f) => match f {
            Some(f) => PathBuf::from(f),
            None => env::current_dir()?,
        },
        None => env::current_dir()?,
    };

    // get file cache size
    let file_cache_size = match args.get(ENV_ARG_FILE_CACHE_SIZE_KEY) {
        Some(c) => match c {
            Some(c) => match c.parse::<usize>() {
                Ok(c) => c * 1024,
                Err(e) => return Err(format!("Invalid cache size: {}", e).into()),
            },
            None => DEFAULT_FILE_CACHE_SIZE,
        },
        None => DEFAULT_FILE_CACHE_SIZE,
    };

    // get log level
    let log_level = match args.get(ENV_ARG_LOG_LEVEL_KEY) {
        Some(l) => match l {
            Some(l) => LogLevel::from(l),
            None => DEFAULT_LOG_LEVEL,
        },
        None => DEFAULT_LOG_LEVEL,
    };

    Ok(Config {
        file_root,
        port,
        file_cache_size,
        log_level,
    })
}

async fn _main() -> Result<(), Box<dyn std::error::Error>> {
    // Get config
    let config = get_config()?;

    // Set log level
    log::set_log_level(config.log_level)?;

    // Log config
    info!(
        "Config:\nport -> {}\nfile root -> {}\nfile cache size -> {}\nlog level -> {}",
        config.port,
        config.file_root.display(),
        fmt_size(config.file_cache_size),
        config.log_level
    );

    // Construct file cache
    let file_cache = FileCache::new(Some(config.file_cache_size));

    // Derive res root folder
    let res_root = config.file_root.join(RES_ROOT_FOLDER);

    // Construct request map if exists
    let request_map = match read_to_string(REQ_MAP_FILE).await {
        Ok(map_file) => {
            let map = RequestMap::parse_str(&map_file)?;
            info!("Map loaded\n{}", &map);
            Some(map)
        }
        Err(e) => match e.kind() {
            std::io::ErrorKind::NotFound => {
                info!("No map file found. Starting without request map...");
                None
            }
            _ => return Err(e.into()),
        },
    };

    // Construct socket
    let sockaddr = format!("0.0.0.0:{}", config.port);
    let listener = TcpListener::bind(&sockaddr).await?;
    info!("socket binded @{}", &sockaddr);

    // Construct context for main loop
    let ctx = Arc::new((file_cache, request_map, res_root));

    // Watcher event
    let watcher_handle = setup_fs_watcher(ctx.clone())?;
    tokio::pin!(watcher_handle); // pin handle in order for main loop to poll it

    // Main loop
    loop {
        // Select between watcher error and listener connection
        let conn = select! {
            res = &mut watcher_handle => Err(res?.unwrap_err()),
            conn = listener.accept() => Ok(conn),
        }?;

        // Accept connection
        let (stream, addr) = match conn {
            Err(e) => {
                error!("Client connection error: {}", e);
                continue;
            }
            Ok(s) => s,
        };
        debug!("connection from: {}", &addr);
        let ctx: Arc<(FileCache, Option<RequestMap>, PathBuf)> = ctx.clone();
        task::spawn(async move {
            let (f_cache, req_map, res_root) = &*ctx;
            let req_map = req_map.as_ref();
            if let Err(e) = handle_connection(&addr, stream, res_root, f_cache, req_map).await {
                error!("Error: {}, {}", &addr, e);
            }
            debug!("connection closed for {}", &addr);
        });
    }
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    if let Err(e) = _main().await {
        error!("{}", e);
    }
}
