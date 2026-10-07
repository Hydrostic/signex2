//! Logical resource identities, Siglus file lookup, and worker-owned CPU decoding.

mod decoder;
mod errors;
mod resolver;
mod worker;

use std::sync::{Arc, mpsc};

pub use decoder::{AssetDecoder, DecodedResource, G00Image, ResourceDecoder};
pub use errors::ResourceError;
pub use resolver::ResourceContext;
pub use worker::ThreadedResourceServer;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResourceId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResourceRevision(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResourceRef {
    pub id: ResourceId,
    pub revision: ResourceRevision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceType {
    Picture,
    NumberAtlas,
    WeatherAtlas,
    Mesh,
    SaveThumbnail,
    Thumbnail,
    Font,
    Mask,
    Sound,
    Voice,
    ObjectMovie,
    SystemMovie,
    Other(u32),
}

pub struct ResourceHandle {
    pub(crate) resource: ResourceRef,
    pub(crate) sender: mpsc::Sender<worker::Command>,
}

impl ResourceHandle {
    pub fn resource(&self) -> ResourceRef {
        self.resource
    }
}

impl std::fmt::Debug for ResourceHandle {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ResourceHandle")
            .field("resource", &self.resource)
            .finish()
    }
}

impl Drop for ResourceHandle {
    fn drop(&mut self) {
        let _ = self.sender.send(worker::Command::Release(self.resource));
    }
}

pub trait ResourceServer: Send + Sync {
    fn open(&self, kind: ResourceType, name: &str) -> Result<ResourceHandle, ResourceError>;
    fn get_data(&self, handle: &ResourceHandle) -> Result<Arc<DecodedResource>, ResourceError>;
}
