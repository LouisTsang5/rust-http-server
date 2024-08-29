mod filecache;
mod getopt;
mod http;
mod requestmap;
mod teewriter;

use filecache::FileCache;
use getopt::getopt;
use http::handle_connection;
use requestmap::RequestMap;
use std::{env, path::PathBuf, sync::Arc};
use tokio::{fs::read_to_string, io::AsyncWriteExt, net::TcpListener, task};

// Constants
const BUFF_INIT_SIZE: usize = 1024; // Referencial init buffer size of all program buffers. All buffers are initialized using multiples of this value.
const DEFAULT_PORT: u16 = 3006;
const RES_ROOT_FOLDER: &str = "res";
const REQ_MAP_FILE: &str = "map.txt";
const ENV_ARG_PORT_KEY: &str = "p";
const ENV_ARG_FILE_ROOT_KEY: &str = "f";

struct Config {
    file_root: PathBuf,
    port: u16,
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

    Ok(Config { file_root, port })
}

async fn _main() -> Result<(), Box<dyn std::error::Error>> {
    // Get config
    let config = get_config()?;
    println!(
        "port: {}\nfile root: {}",
        config.port,
        config.file_root.display()
    );

    // Construct file cache
    let file_cache = FileCache::new();

    // Derive res root folder
    let res_root = config.file_root.join(RES_ROOT_FOLDER);

    // Construct request map if exists
    let request_map = match read_to_string(REQ_MAP_FILE).await {
        Ok(map_file) => {
            let map = RequestMap::parse_str(&map_file)?;
            println!("Map loaded\n{}", &map);
            Some(map)
        }
        Err(e) => match e.kind() {
            std::io::ErrorKind::NotFound => {
                println!("No map file found. Starting without request map...");
                None
            }
            _ => return Err(e.into()),
        },
    };

    // Construct socket
    let sockaddr = format!("0.0.0.0:{}", config.port);
    let listener = TcpListener::bind(&sockaddr).await?;
    println!("socket binded @{}", &sockaddr);

    // Construct context for main loop
    let ctx = Arc::new((file_cache, request_map, res_root));

    // Main loop
    loop {
        let (mut stream, addr) = match listener.accept().await {
            Err(e) => {
                eprintln!("Client connection error: {}", e);
                continue;
            }
            Ok(s) => s,
        };
        println!("connection from: {}", &addr);
        let ctx = ctx.clone();
        task::spawn(async move {
            let (file_cache, request_map, res_root) = &*ctx;
            if let Err(e) =
                handle_connection(&mut stream, res_root, file_cache, request_map.as_ref()).await
            {
                eprintln!("Error: {}, {}", &addr, e);
            }
            if let Err(e) = stream.shutdown().await {
                eprintln!("Error shutting down connection: {}, {}", &addr, e);
            }
            println!("connection closed for {}", &addr);
        });
    }
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    if let Err(e) = _main().await {
        eprintln!("{}", e);
    }
}
