use crate::io_error;
use share_domain::Error;
use std::fs::File;

/// Stable native identity without retaining one descriptor for every scanned file.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub(crate) struct Identity(u64, u64);
impl Identity {
    #[cfg(unix)]
    pub fn of(file: &File) -> Result<Self, Error> {
        use std::os::unix::fs::MetadataExt;
        let metadata = file.metadata().map_err(io_error)?;
        Ok(Self(metadata.dev(), metadata.ino()))
    }
    #[cfg(windows)]
    pub fn of(file: &File) -> Result<Self, Error> {
        let info = winapi_util::file::information(file).map_err(io_error)?;
        Ok(Self(info.volume_serial_number(), info.file_index()))
    }
    #[cfg(not(any(unix, windows)))]
    pub fn of(_: &File) -> Result<Self, Error> {
        Err(Error::Unsupported)
    }
}
