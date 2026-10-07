use std::path::PathBuf;

use crate::{ResourceId, ResourceRef, ResourceType};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ResourceError {
    #[error("invalid resource context")]
    InvalidContext,
    #[error("invalid resource name")]
    InvalidName,
    #[error("resource {kind:?} {name:?} not found")]
    NotFound {
        kind: ResourceType,
        name: String,
        searched: Vec<PathBuf>,
    },
    #[error("I/O error at {}: {message}", path.display())]
    Io { path: PathBuf, message: String },
    #[error("unknown resource {0:?}")]
    UnknownResource(ResourceRef),
    #[error("stale revision {requested:?}; current is {current:?}")]
    StaleRevision {
        requested: ResourceRef,
        current: ResourceRef,
    },
    #[error("decode failed for {resource:?} at {}: {message}", path.display())]
    DecodeFailed {
        resource: ResourceRef,
        path: PathBuf,
        message: String,
    },
    #[error("resource worker stopped")]
    WorkerStopped,
    #[error("a resource server is already running")]
    AlreadyRunning,
    #[error("revision overflow for {0:?}")]
    RevisionOverflow(ResourceId),
    #[error("resource ID collision for {0:?}")]
    IdCollision(ResourceId),
}
