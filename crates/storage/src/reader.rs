use crate::Reader;
use share_domain::Error;
use std::{
    io::SeekFrom,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncSeek, AsyncSeekExt, ReadBuf};

trait ReadSeek: AsyncRead + AsyncSeek + Send + Unpin {}
impl<T: AsyncRead + AsyncSeek + Send + Unpin> ReadSeek for T {}

enum Kind {
    Stream(Reader),
    Seekable(Pin<Box<dyn ReadSeek>>),
}

/// An opened object's reader, with optional same-handle positioning. Providers
/// that only implement AsyncRead remain supported through the stream variant.
pub struct FileReader(Kind);

impl FileReader {
    pub fn seekable(reader: impl AsyncRead + AsyncSeek + Send + Unpin + 'static) -> Self {
        Self(Kind::Seekable(Box::pin(reader)))
    }

    /// Advance from the current position without reopening a pathname. Reject a
    /// shortened prefix for both variants; seeking beyond EOF otherwise succeeds
    /// and would falsely declare a satisfiable Range with an empty response.
    pub async fn advance(&mut self, bytes: u64) -> Result<(), Error> {
        if bytes == 0 {
            return Ok(());
        }
        match &mut self.0 {
            Kind::Seekable(reader) => {
                let position = reader.stream_position().await.map_err(|_| Error::Io)?;
                let next = position.checked_add(bytes).ok_or(Error::Conflict)?;
                let end = reader.seek(SeekFrom::End(0)).await.map_err(|_| Error::Io)?;
                if next > end {
                    return Err(Error::Conflict);
                }
                let actual = reader
                    .seek(SeekFrom::Start(next))
                    .await
                    .map_err(|_| Error::Io)?;
                if actual != next {
                    return Err(Error::Conflict);
                }
            }
            Kind::Stream(reader) => {
                let copied = tokio::io::copy(&mut reader.take(bytes), &mut tokio::io::sink())
                    .await
                    .map_err(|_| Error::Io)?;
                if copied != bytes {
                    return Err(Error::Conflict);
                }
            }
        }
        Ok(())
    }
}

impl From<Reader> for FileReader {
    fn from(reader: Reader) -> Self {
        Self(Kind::Stream(reader))
    }
}

impl From<FileReader> for Reader {
    fn from(reader: FileReader) -> Self {
        match reader.0 {
            Kind::Stream(reader) => reader,
            Kind::Seekable(reader) => reader,
        }
    }
}

impl AsyncRead for FileReader {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        match &mut self.get_mut().0 {
            Kind::Stream(reader) => reader.as_mut().poll_read(cx, buf),
            Kind::Seekable(reader) => reader.as_mut().poll_read(cx, buf),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn readers() -> [FileReader; 2] {
        let stream: Reader = Box::pin(Cursor::new(b"abcdef".to_vec()));
        [
            stream.into(),
            FileReader::seekable(Cursor::new(b"abcdef".to_vec())),
        ]
    }

    #[tokio::test]
    async fn both_reader_kinds_advance_relative_to_the_current_position() {
        for mut reader in readers() {
            let mut prefix = [0; 2];
            reader.read_exact(&mut prefix).await.unwrap();
            assert_eq!(&prefix, b"ab");
            reader.advance(2).await.unwrap();
            let mut suffix = Vec::new();
            reader.read_to_end(&mut suffix).await.unwrap();
            assert_eq!(suffix, b"ef");
        }
    }

    #[tokio::test]
    async fn both_reader_kinds_reject_a_prefix_beyond_the_opened_objects_end() {
        for mut reader in readers() {
            assert!(matches!(reader.advance(7).await, Err(Error::Conflict)));
        }
    }
}
