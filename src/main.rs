use std::{
    error::Error,
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
};

fn handle_connection(mut stream: TcpStream) -> Result<(), Box<dyn Error>> {
    let buf_reader = BufReader::new(&mut stream);

    // Read the header
    let http_request: Vec<String> = buf_reader
        .lines()
        .map(|result| result.unwrap())
        .take_while(|line| !line.is_empty())
        .collect();
    for line in &http_request {
        println!("{}", line);
    }

    // Get content length
    let mut content_length: Option<usize> = None;
    for line in &http_request {
        const KEY: &str = "Content-Length: ";
        if line.starts_with(KEY) {
            content_length = Some(line[KEY.len()..].parse()?)
        }
    }
    if let None = content_length {
        return Err("Cannot find content length".into());
    }

    let content_length = content_length.unwrap();
    println!("Content Length = {}", content_length);

    // Read body
    let mut buff = vec![0u8; content_length];
    stream.read_exact(&mut buff)?;
    let body = String::from_utf8_lossy(&buff);
    println!("{}", body);

    // Response
    let response = "HTTP/1.1 200 OK\r\n\r\n";
    stream.write_all(response.as_bytes()).unwrap();

    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind("0.0.0.0:80")?;
    for stream in listener.incoming() {
        let stream = match stream {
            Err(e) => {
                eprintln!("{}", e);
                continue;
            }
            Ok(s) => s,
        };
        handle_connection(stream)?;
    }
    Ok(())
}
