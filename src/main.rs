use std::{
    collections::HashMap,
    error::Error,
    fs::File,
    io::{self, stdout, BufReader, Cursor, Read, Write},
    net::{TcpListener, TcpStream},
    path::Path,
};

const HEADER_BUFF_INIT_SIZE: usize = 1024;
const RES_ROOT_FOLDER: &str = "res";

fn read_headers_buff<R: Read>(stream: &mut R) -> Result<Vec<u8>, io::Error> {
    let mut res = Vec::with_capacity(HEADER_BUFF_INIT_SIZE);
    let mut t_bytes_read = 0;
    const END_OF_HEADER: &[u8] = b"\r\n\r\n";
    const READ_SIZE: usize = 1;
    while t_bytes_read < END_OF_HEADER.len()
        || &res[t_bytes_read - END_OF_HEADER.len()..] != END_OF_HEADER
    {
        let mut buff = [0u8; READ_SIZE];
        let bytes_read = stream.read(&mut buff)?;
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

pub struct TeeWriter<L, R> {
    left: L,
    right: R,
}

impl<L: Write, R: Write> TeeWriter<L, R> {
    fn new(left: L, right: R) -> Self {
        Self { left, right }
    }
}

impl<L: Write, R: Write> Write for TeeWriter<L, R> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let n = self.left.write(&buf[..])?;
        self.right.write_all(&buf[..n])?;
        Ok(n)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.left.flush()?;
        self.right.flush()?;
        Ok(())
    }
}

fn handle_connection(stream: TcpStream) -> Result<(), Box<dyn Error>> {
    // Split stream to a buffered reader and a writer
    let mut r_stream = BufReader::new(stream.try_clone()?);
    let w_stream = stream;

    // Read the header
    let http_request = String::from_utf8_lossy(&read_headers_buff(&mut r_stream)?).to_string();
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
        r_stream.read_exact(&mut buff)?;
        let body = String::from_utf8_lossy(&buff);
        println!("{}", body);
    }
    // Print a new line
    println!("");

    // Open res file
    let path = match http_request.path.ends_with("/") {
        true => format!("{}index", http_request.path),
        false => http_request.path,
    };
    let path = Path::new(RES_ROOT_FOLDER).join(format!(".{}", &path));
    let (status, res_body, res_body_len) = match path.exists() {
        true => {
            let status = "200 OK";
            let res_body = File::open(path)?;
            let res_body_len = res_body.metadata()?.len() as usize;
            (status, Box::new(res_body) as Box<dyn Read>, res_body_len)
        }
        false => {
            let status = "404 Not Found";
            let msg = b"NOT FOUND";
            let res_body = Cursor::new(msg);
            let res_body_len = msg.len();
            (status, Box::new(res_body) as Box<dyn Read>, res_body_len)
        }
    };
    let mut res_header = String::with_capacity(HEADER_BUFF_INIT_SIZE);
    res_header.push_str(&format!("HTTP/1.1 {}\r\n", status).to_string());
    res_header.push_str(&format!("Content-Length: {}\r\n", res_body_len));
    res_header.push_str("\r\n");
    let mut res = Cursor::new(res_header).chain(res_body);

    // Write to both stream and console
    let mut tee_writer = TeeWriter::new(w_stream, stdout());
    io::copy(&mut res, &mut tee_writer)?;
    println!("");

    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind("0.0.0.0:80")?;
    println!("socket binded");
    for stream in listener.incoming() {
        println!("connection accepted");
        let stream = match stream {
            Err(e) => {
                eprintln!("Error: {}", e);
                continue;
            }
            Ok(s) => s,
        };
        if let Err(e) = handle_connection(stream) {
            eprintln!("Error: {}", e);
        }
    }
    Ok(())
}
