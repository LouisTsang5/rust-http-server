mod filecache;
mod http;
mod requestmap;
mod teewriter;

use filecache::FileCache;
use http::handle_connection;
use requestmap::RequestMap;
use std::{env, path::Path, sync::Arc};
use tokio::{fs::read_to_string, io::AsyncWriteExt, net::TcpListener, task};

// Constants
const BUFF_INIT_SIZE: usize = 1024; // Referencial init buffer size of all program buffers. All buffers are initialized using multiples of this value.
const DEFAULT_PORT: u16 = 3006;
const RES_ROOT_FOLDER: &str = "res";
const REQ_MAP_FILE: &str = "map.txt";

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Construct file cache
    let file_cache = Arc::new(FileCache::new());

    // Get the root folder of files
    let exec_path = env::current_exe()?;
    let file_root = exec_path.parent().unwrap().join(RES_ROOT_FOLDER);
    let file_root: Arc<Path> = file_root.into();
    println!("file root: {}", file_root.display());

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
    let request_map = Arc::new(request_map);

    // Construct socket
    let port = match env::args_os()
        .map(|arg| arg.to_string_lossy().into_owned())
        .nth(1)
    {
        Some(p) => match p.parse::<u16>() {
            Ok(p) => Some(p),
            Err(e) => return Err(format!("Invalid port: {}", e).into()),
        },
        None => None,
    };
    let sockaddr = format!("0.0.0.0:{}", port.unwrap_or(DEFAULT_PORT));
    let listener = TcpListener::bind(&sockaddr).await?;
    println!("socket binded @{}", &sockaddr);

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
        let file_cache = file_cache.clone();
        let request_map = request_map.clone();
        let file_root = file_root.clone();
        task::spawn(async move {
            let request_map = request_map.as_ref().as_ref();
            let filecache = file_cache.as_ref();
            if let Err(e) = handle_connection(&mut stream, &file_root, filecache, request_map).await
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
