use crate::filecache::FileCache;
use crate::requestmap::RequestMap;
use crate::teewriter::tee_write;
use std::{borrow::Cow, collections::HashMap, io::Cursor, path::Path};
use tokio::{
    io::{self, stdout, AsyncRead, AsyncReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
};

const HEADER_BUFF_INIT_SIZE: usize = crate::BUFF_INIT_SIZE * 8;

async fn read_headers_buff<R: AsyncRead + Unpin>(stream: &mut R) -> Result<Vec<u8>, io::Error> {
    let mut res = Vec::with_capacity(HEADER_BUFF_INIT_SIZE);
    let mut t_bytes_read = 0;
    const END_OF_HEADER: &[u8] = b"\r\n\r\n";
    const READ_SIZE: usize = 1;
    while t_bytes_read < END_OF_HEADER.len()
        || &res[t_bytes_read - END_OF_HEADER.len()..] != END_OF_HEADER
    {
        let mut buff = [0u8; READ_SIZE];
        let bytes_read = stream.read(&mut buff).await?;

        // Check if end of file
        if bytes_read <= 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Unexpected end of file",
            ));
        }

        t_bytes_read += bytes_read;
        res.extend_from_slice(&buff);
    }
    Ok(res)
}

struct HttpRequest {
    method: String,
    path: String,
    protocol: String,
    headers: HashMap<String, String>,
}

fn parse_http(header_lines: &[&str]) -> HttpRequest {
    // Parse the start line
    let (method, path, protocol) = {
        let start_line_items: Vec<&str> = header_lines[0].split(' ').collect();
        (
            start_line_items[0].to_string(),
            start_line_items[1].to_string(),
            start_line_items[2].to_string(),
        )
    };

    // Parse the headers
    let mut headers = HashMap::new();
    for line in &header_lines[1..] {
        let key_val: Vec<&str> = line.split(':').collect();
        let key = key_val[0].trim().to_string();
        let val = key_val[1].trim().to_string();
        headers.insert(key, val);
    }

    HttpRequest {
        method,
        path,
        protocol,
        headers,
    }
}

pub async fn handle_connection(
    stream: &mut TcpStream,
    res_file_root: &Path,
    file_cache: &FileCache,
    request_map: Option<&RequestMap>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Split stream to a buffered reader and a writer
    let (r_stream, mut w_stream) = stream.split();
    let mut r_stream = BufReader::new(r_stream);

    // Read the header
    let header_buff = read_headers_buff(&mut r_stream).await?;
    let http_request = String::from_utf8_lossy(&header_buff);
    let http_request: Vec<&str> = http_request
        .lines()
        .take_while(|line| !line.is_empty())
        .collect();
    let http_request = parse_http(&http_request);
    println!(
        "{} {} {}",
        &http_request.method, &http_request.path, &http_request.protocol
    );
    for (key, val) in &http_request.headers {
        println!("{}: {}", key, val);
    }

    // Read the body if request is POST
    if &http_request.method == "POST" {
        // Line break for body
        println!("");

        // Get content length
        let content_length = http_request.headers.get("Content-Length");
        if let None = content_length {
            return Err("Cannot find content length".into());
        }
        let content_length = match content_length.unwrap().parse::<usize>() {
            Ok(l) => l,
            Err(e) => return Err(format!("Failed read content length: {}", e).into()),
        };

        // Read the body
        let mut buff = vec![0u8; content_length];
        r_stream.read_exact(&mut buff).await?;
        let body = String::from_utf8_lossy(&buff);
        println!("{}", body);
    }
    // Print a new line
    println!("");

    // Try to find the file from the map, if not exists, use the http request path as it is
    let file_path = match request_map {
        Some(map) => map.get(&http_request.path),
        None => None,
    };
    let file_path = match file_path {
        Some(p) => p,
        None => match http_request.path.starts_with('/') {
            true => Path::new(&http_request.path[1..]), // Remove the leading slash
            false => Path::new(&http_request.path),
        },
    };

    // Check if the path is a directory, if so, use the index file
    let file_path = res_file_root.join(file_path);
    let file_path = match file_path.is_dir() {
        true => Cow::Owned(file_path.join("index")),
        false => Cow::Borrowed(&file_path),
    };

    // Open res file
    println!("Opening file: {}", &file_path.as_path().display());
    let mut file = match file_cache.open(&file_path).await {
        Ok(f) => Some(f),
        Err(e) => match e.kind() {
            io::ErrorKind::NotFound => {
                println!("File not found: {}", &file_path.as_path().display());
                None
            }
            _ => return Err(e.into()),
        },
    };

    // Print a new line
    println!("");

    // Write the response
    const NOT_FOUND_STATUS: &str = "404 Not Found";
    const NOT_FOUND_MSG: &str = "NOT FOUND";
    const OK_STATUS: &str = "200 OK";
    let mut res = String::with_capacity(HEADER_BUFF_INIT_SIZE);
    res.push_str(&format!(
        // Write the status line
        "HTTP/1.1 {}\r\n",
        match &file {
            Some(_) => OK_STATUS,
            None => NOT_FOUND_STATUS,
        }
    ));
    res.push_str(&format!(
        // Write the content length
        "Content-Length: {}\r\n",
        match &file {
            Some(f) => f.len(),
            None => NOT_FOUND_MSG.len(),
        }
    ));
    res.push_str("\r\n"); // End of header

    // convert header to stream and chain with body of either a file or a string
    let mut not_found_body = Cursor::new(NOT_FOUND_MSG.as_bytes());
    let mut res = AsyncReadExt::chain(
        Cursor::new(res),
        match &mut file {
            Some(f) => f as &mut (dyn AsyncRead + Unpin + Send),
            None => &mut not_found_body,
        },
    );

    // Write to both stream and console
    let mut stdout = stdout();
    tee_write(
        &mut res,
        &mut [
            &mut w_stream as &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
            &mut stdout as &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        ],
    )
    .await?;
    stdout.flush().await?;
    println!("");

    Ok(())
}
