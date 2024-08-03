use futures_util::{AsyncRead, AsyncWrite};
use pin_project_lite::pin_project;

pin_project! {
	pub struct CombineRW<R, W> {
		#[pin]
		read: R,
		#[pin]
		write: W,
	}
}

impl<R, W> CombineRW<R, W> {
	pub fn new(read: R, write: W) -> Self {
		Self { read, write }
	}
}

impl<R: AsyncRead, W> AsyncRead for CombineRW<R, W> {
	fn poll_read(
		self: std::pin::Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
		buf: &mut [u8],
	) -> std::task::Poll<std::io::Result<usize>> {
		self.project().read.poll_read(cx, buf)
	}

	fn poll_read_vectored(
		self: std::pin::Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
		bufs: &mut [std::io::IoSliceMut<'_>],
	) -> std::task::Poll<std::io::Result<usize>> {
		self.project().read.poll_read_vectored(cx, bufs)
	}
}

impl<R, W: AsyncWrite> AsyncWrite for CombineRW<R, W> {
	fn poll_write(
		self: std::pin::Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
		buf: &[u8],
	) -> std::task::Poll<std::io::Result<usize>> {
		self.project().write.poll_write(cx, buf)
	}

	fn poll_write_vectored(
		self: std::pin::Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
		bufs: &[std::io::IoSlice<'_>],
	) -> std::task::Poll<std::io::Result<usize>> {
		self.project().write.poll_write_vectored(cx, bufs)
	}

	fn poll_flush(
		self: std::pin::Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
	) -> std::task::Poll<std::io::Result<()>> {
		self.project().write.poll_flush(cx)
	}

	fn poll_close(
		self: std::pin::Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
	) -> std::task::Poll<std::io::Result<()>> {
		self.project().write.poll_close(cx)
	}
}
