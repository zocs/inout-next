use crate::identity::Identity;
use crate::io_error;
use cap_fs_ext::{DirExt, FollowSymlinks, OpenOptionsFollowExt};
use cap_std::fs::{Dir, File, Metadata, OpenOptions};
use share_domain::Error;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

#[derive(Default)]
pub(crate) struct ProtectedObjects {
    objects: Mutex<HashMap<Identity, usize>>,
}
pub(crate) struct Protection {
    registry: Arc<ProtectedObjects>,
    objects: Vec<Identity>,
}
fn directory_handle(dir: &Dir) -> Result<Identity, Error> {
    Identity::of(&dir.try_clone().map_err(io_error)?.into_std_file())
}
fn file_handle(file: &File) -> Result<Identity, Error> {
    Identity::of(&file.try_clone().map_err(io_error)?.into_std())
}
impl ProtectedObjects {
    pub fn register(self: &Arc<Self>, dir: &Dir, file: &File) -> Result<Protection, Error> {
        let objects = vec![directory_handle(dir)?, file_handle(file)?];
        let mut registry = self.objects.lock().map_err(|_| Error::Conflict)?;
        for object in &objects {
            *registry.entry(*object).or_default() += 1;
        }
        Ok(Protection {
            registry: self.clone(),
            objects,
        })
    }
    fn check(&self, handle: &Identity) -> Result<(), Error> {
        if self
            .objects
            .lock()
            .map_err(|_| Error::Conflict)?
            .contains_key(handle)
        {
            Err(Error::Forbidden)
        } else {
            Ok(())
        }
    }
    pub fn check_dir(&self, dir: &Dir) -> Result<(), Error> {
        self.check(&directory_handle(dir)?)
    }
    pub fn check_file(&self, file: &File) -> Result<(), Error> {
        self.check(&file_handle(file)?)
    }
    pub fn check_entry(&self, dir: &Dir, name: &str, metadata: &Metadata) -> Result<(), Error> {
        if self.objects.lock().map_err(|_| Error::Conflict)?.is_empty() {
            return Ok(());
        }
        if metadata.is_dir() {
            return self.check_dir(&dir.open_dir_nofollow(name).map_err(io_error)?);
        }
        let mut options = OpenOptions::new();
        options.read(true).follow(FollowSymlinks::No);
        self.check_file(&dir.open_with(name, &options).map_err(io_error)?)
    }
}
impl Drop for Protection {
    fn drop(&mut self) {
        if let Ok(mut registry) = self.registry.objects.lock() {
            for object in &self.objects {
                if let Some(count) = registry.get_mut(object) {
                    *count -= 1;
                    if *count == 0 {
                        registry.remove(object);
                    }
                }
            }
        }
    }
}
