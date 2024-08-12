use std::{
    pin::Pin,
    task::{Context, Poll},
};

use tokio::io::AsyncWrite;

pub struct TeeWriter<L, R> {
    left: L,
    right: R,
}

impl<L: AsyncWrite, R: AsyncWrite> TeeWriter<L, R> {
    pub fn new(left: L, right: R) -> Self {
        Self { left, right }
    }
}

impl<L: AsyncWrite + Unpin, R: AsyncWrite + Unpin> AsyncWrite for TeeWriter<L, R> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        // Make a list of poll write futures
        let this = self.get_mut();
        let w_futures = [
            Pin::new(&mut this.left).poll_write(cx, buf),
            Pin::new(&mut this.right).poll_write(cx, buf),
        ];

        // iterate over each future
        let mut num_bytes_written = 0;
        for result in w_futures.into_iter() {
            match result {
                Poll::Ready(Ok(n)) => num_bytes_written = n, // continue to next writer future if current future is ready
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)), // return error if current future has error
                Poll::Pending => return Poll::Pending, // return pending if current future is pending
            }
        }

        // return the number of bytes written
        Poll::Ready(Ok(num_bytes_written))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), std::io::Error>> {
        // Make a list of poll flush futures
        let this = self.get_mut();
        let w_futures = [
            Pin::new(&mut this.left).poll_flush(cx),
            Pin::new(&mut this.right).poll_flush(cx),
        ];

        // iterate over each future and return error if any future has error
        for result in w_futures.into_iter() {
            if let Poll::Ready(Err(e)) = result {
                return Poll::Ready(Err(e));
            }
        }

        // return ready if all futures are ready
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        // Make a list of poll shutdown futures
        let this = self.get_mut();
        let w_futures = [
            Pin::new(&mut this.left).poll_shutdown(cx),
            Pin::new(&mut this.right).poll_shutdown(cx),
        ];

        // iterate over each future and return error if any future has error
        for result in w_futures.into_iter() {
            if let Poll::Ready(Err(e)) = result {
                return Poll::Ready(Err(e));
            }
        }

        // return ready if all futures are ready
        Poll::Ready(Ok(()))
    }
}
