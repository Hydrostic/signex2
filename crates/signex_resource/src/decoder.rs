use std::{
    any::Any,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{ResourceError, ResourceRef, ResourceType};

pub use signex_asset::G00Image;

pub enum DecodedResource {
    G00(G00Image),
    Other(Box<dyn Any + Send + Sync>),
}

impl std::fmt::Debug for DecodedResource {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::G00(image) => formatter.debug_tuple("G00").field(image).finish(),
            Self::Other(_) => formatter.write_str("Other(..)"),
        }
    }
}

pub trait ResourceDecoder: Send + Sync + 'static {
    fn decode(
        &self,
        kind: ResourceType,
        path: &Path,
        bytes: &[u8],
    ) -> Result<DecodedResource, String>;
}

#[derive(Debug, Default)]
pub struct AssetDecoder;

impl ResourceDecoder for AssetDecoder {
    fn decode(
        &self,
        kind: ResourceType,
        path: &Path,
        bytes: &[u8],
    ) -> Result<DecodedResource, String> {
        if matches!(
            kind,
            ResourceType::Picture
                | ResourceType::NumberAtlas
                | ResourceType::WeatherAtlas
                | ResourceType::Mask
        ) {
            return signex_asset::decode_g00(bytes)
                .map(DecodedResource::G00)
                .map_err(|error| error.to_string());
        }
        Err(format!("no decoder for {kind:?} at {}", path.display()))
    }
}

pub(crate) type DecodeResult = Result<Arc<DecodedResource>, ResourceError>;

pub(crate) struct DecodeJob {
    pub resource: ResourceRef,
    pub kind: ResourceType,
    pub path: PathBuf,
}

pub(crate) fn decode_job(
    decoder: &dyn ResourceDecoder,
    job: DecodeJob,
) -> (ResourceRef, DecodeResult) {
    let DecodeJob {
        resource,
        kind,
        path,
    } = job;
    let result = fs::read(&path)
        .map_err(|error| ResourceError::DecodeFailed {
            resource,
            path: path.clone(),
            message: error.to_string(),
        })
        .and_then(|bytes| {
            decoder
                .decode(kind, &path, &bytes)
                .map(Arc::new)
                .map_err(|message| ResourceError::DecodeFailed {
                    resource,
                    path,
                    message,
                })
        });
    (resource, result)
}
