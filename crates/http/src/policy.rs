use crate::auth::Auth;
use serde::Serialize;
use share_domain::{Error, ModuleRegistry, Permissions};
use std::sync::{Arc, RwLock};

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PolicyStatus {
    pub revision: u64,
    pub permissions: Permissions,
    pub auth_required: bool,
}

/// Credentials are deliberately neither Debug nor Serialize.
pub enum KeyUpdate {
    Keep,
    Replace(String),
    Remove,
}

pub struct PolicyUpdate {
    pub expected_revision: u64,
    pub permissions: Permissions,
    pub key: KeyUpdate,
}

pub(crate) struct PolicySnapshot {
    pub revision: u64,
    pub permissions: Permissions,
    pub auth: Arc<Auth>,
}

impl PolicySnapshot {
    fn status(&self) -> PolicyStatus {
        PolicyStatus {
            revision: self.revision,
            permissions: self.permissions.clone(),
            auth_required: self.auth.required(),
        }
    }
}

struct Control {
    registry: ModuleRegistry,
    snapshot: RwLock<Arc<PolicySnapshot>>,
}

/// Application-side authority, never registered as an HTTP setting endpoint.
/// Publication under one lock is the admission/update linearization point.
#[derive(Clone)]
pub struct PolicyControl(Arc<Control>);

impl PolicyControl {
    pub(crate) fn new(
        registry: ModuleRegistry,
        permissions: Permissions,
        key: Option<&str>,
    ) -> Result<Self, Error> {
        if key.is_some_and(|key| key.trim().is_empty()) {
            return Err(Error::Conflict);
        }
        let auth = Arc::new(Auth::new(key));
        validate(&registry, &permissions, auth.required())?;
        Ok(Self(Arc::new(Control {
            registry,
            snapshot: RwLock::new(Arc::new(PolicySnapshot {
                revision: 1,
                permissions,
                auth,
            })),
        })))
    }

    pub(crate) fn snapshot(&self) -> Arc<PolicySnapshot> {
        // The stored Arc is replaced only after complete validation; recovery
        // from a poisoned lock still observes a complete immutable snapshot.
        self.0
            .snapshot
            .read()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }

    pub fn status(&self) -> PolicyStatus {
        self.snapshot().status()
    }

    pub fn update(&self, update: PolicyUpdate) -> Result<PolicyStatus, Error> {
        let mut current = self
            .0
            .snapshot
            .write()
            .unwrap_or_else(|error| error.into_inner());
        if update.expected_revision != current.revision {
            return Err(Error::Conflict);
        }
        let auth = match update.key {
            KeyUpdate::Keep => current.auth.clone(),
            KeyUpdate::Replace(key) => {
                if key.trim().is_empty() {
                    return Err(Error::Conflict);
                }
                Arc::new(Auth::new(Some(&key)))
            }
            KeyUpdate::Remove => Arc::new(Auth::new(None)),
        };
        validate(&self.0.registry, &update.permissions, auth.required())?;
        let next = Arc::new(PolicySnapshot {
            revision: current.revision.checked_add(1).ok_or(Error::Conflict)?,
            permissions: update.permissions,
            auth,
        });
        let status = next.status();
        *current = next;
        Ok(status)
    }
}

fn validate(
    registry: &ModuleRegistry,
    permissions: &Permissions,
    keyed: bool,
) -> Result<(), Error> {
    if ((permissions.list || permissions.download) && !registry.has("files"))
        || (permissions.upload && !registry.has("uploads"))
        || ((permissions.manage || permissions.rename || permissions.delete)
            && !registry.has("manage"))
    {
        return Err(Error::Unsupported);
    }
    if permissions.delete && !keyed {
        return Err(Error::Forbidden);
    }
    Ok(())
}
