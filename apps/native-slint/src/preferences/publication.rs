//! Coordinate Windows atomic replacements through a thread-owned kernel mutex.
use std::{
    fs::{File, OpenOptions},
    io,
    os::windows::{fs::OpenOptionsExt, io::AsRawHandle},
    path::Path,
};
use windows_sys::Win32::{
    Foundation::{CloseHandle, HANDLE, WAIT_ABANDONED, WAIT_OBJECT_0, WAIT_TIMEOUT},
    Storage::FileSystem::{
        GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION, FILE_FLAG_BACKUP_SEMANTICS,
        FILE_READ_ATTRIBUTES, SYNCHRONIZE,
    },
    System::Threading::{CreateMutexExW, ReleaseMutex, WaitForSingleObject, MUTEX_MODIFY_STATE},
};

const WAIT_MS: u32 = 5000;

pub(super) struct Guard {
    // Keep the directory identity alive for the entire publication.
    _directory: File,
    mutex: HANDLE,
    owned: bool,
}

impl Guard {
    pub(super) fn acquire(directory: &Path) -> io::Result<Self> {
        let directory = OpenOptions::new()
            .access_mode(FILE_READ_ATTRIBUTES)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
            .open(directory)?;
        let mut identity = BY_HANDLE_FILE_INFORMATION::default();
        // The open directory owns this valid handle; the output has its exact
        // Win32 layout and remains initialized even if the call fails.
        if unsafe { GetFileInformationByHandle(directory.as_raw_handle(), &mut identity) } == 0 {
            return Err(io::Error::last_os_error());
        }
        // Directory identity makes aliases and differing pathname case use the
        // same lock. Global names also coordinate instances in other sessions.
        let name: Vec<u16> = format!(
            "Global\\inout-next-native-preferences-{:08x}-{:08x}-{:08x}",
            identity.dwVolumeSerialNumber, identity.nFileIndexHigh, identity.nFileIndexLow
        )
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
        // The name is terminated and live for this call. Default security keeps
        // the object under the caller's access rules; request only wait/release.
        let mutex = unsafe {
            CreateMutexExW(
                std::ptr::null(),
                name.as_ptr(),
                0,
                SYNCHRONIZE | MUTEX_MODIFY_STATE,
            )
        };
        if mutex.is_null() {
            return Err(io::Error::last_os_error());
        }
        let mut guard = Self {
            _directory: directory,
            mutex,
            owned: false,
        };
        // This valid mutex handle is owned by guard. A bounded wait preserves a
        // complete previous document if another publisher stops making progress.
        match unsafe { WaitForSingleObject(mutex, WAIT_MS) } {
            WAIT_OBJECT_0 | WAIT_ABANDONED => {
                // Abandonment grants ownership too; stages were never shared.
                guard.owned = true;
                Ok(guard)
            }
            WAIT_TIMEOUT => Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "another preference publication did not finish",
            )),
            _ => Err(io::Error::last_os_error()),
        }
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        // HANDLE is not Send: acquisition and release stay on the owning thread.
        // Every constructed guard owns a valid mutex handle, including failures
        // to acquire it; only successful acquisition needs ReleaseMutex.
        unsafe {
            if self.owned {
                let _ = ReleaseMutex(self.mutex);
            }
            let _ = CloseHandle(self.mutex);
        }
    }
}
