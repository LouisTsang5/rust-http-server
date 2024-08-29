use std::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const TEE_WRITER_BUFF_SIZE: usize = crate::BUFF_INIT_SIZE * 8;

pub async fn tee_write<R: AsyncReadExt + Unpin, W: AsyncWriteExt + Unpin>(
    mut src: R,
    out_list: &mut [W],
) -> io::Result<usize> {
    // Define buffer & total bytes read
    let mut buf = [0u8; TEE_WRITER_BUFF_SIZE];
    let mut t_bytes_read = 0usize;

    // Pipe data loop
    loop {
        // Read from src
        let bytes_read = src.read(&mut buf).await?;

        // Break if eof
        if bytes_read <= 0 {
            break;
        }

        // Update total bytes read
        t_bytes_read += bytes_read;

        // Write to all out
        for out in out_list.iter_mut() {
            out.write_all(&buf[..bytes_read]).await?;
        }
    }

    Ok(t_bytes_read)
}
