use std::{
    fs, io,
    path::{Path, PathBuf},
};

use signex_util::is_relative;

use crate::{ResourceError, ResourceType};

#[derive(Debug, Clone)]
pub struct ResourceContext {
    pub(crate) root: PathBuf,
    pub(crate) save_dir: Option<PathBuf>,
    pub(crate) appends: Vec<PathBuf>,
    pub(crate) current: usize,
}

impl ResourceContext {
    pub fn new(
        root: PathBuf,
        appends: Vec<PathBuf>,
        current: usize,
    ) -> Result<Self, ResourceError> {
        if !root.is_absolute() || appends.is_empty() || current >= appends.len() {
            return Err(ResourceError::InvalidContext);
        }
        if appends
            .iter()
            .any(|append| !append.as_os_str().is_empty() && !is_relative(append))
        {
            return Err(ResourceError::InvalidContext);
        }
        Ok(Self {
            root,
            save_dir: None,
            appends,
            current,
        })
    }

    pub fn with_save_dir(mut self, save_dir: PathBuf) -> Result<Self, ResourceError> {
        if !save_dir.is_absolute() {
            return Err(ResourceError::InvalidContext);
        }
        self.save_dir = Some(save_dir);
        Ok(self)
    }

    pub(crate) fn resolve(&self, kind: ResourceType, name: &str) -> Result<PathBuf, ResourceError> {
        if name.is_empty() || !is_relative(Path::new(name)) {
            return Err(ResourceError::InvalidName);
        }

        let (roots, candidates) = match kind {
            ResourceType::SaveThumbnail | ResourceType::Thumbnail => {
                let save_dir = self
                    .save_dir
                    .as_ref()
                    .ok_or(ResourceError::InvalidContext)?;
                let number = name
                    .parse::<i32>()
                    .map_err(|_| ResourceError::InvalidName)?;
                let padded = if kind == ResourceType::SaveThumbnail {
                    format!("{number:04}")
                } else {
                    format!("{number:010}")
                };
                (vec![save_dir.clone()], candidates(kind, &padded)?)
            }
            _ => (
                self.appends[self.current..]
                    .iter()
                    .map(|append| self.root.join(append))
                    .collect(),
                candidates(kind, name)?,
            ),
        };
        let mut searched = Vec::new();
        for root in roots {
            for candidate in &candidates {
                let path = root.join(candidate);
                searched.push(path.clone());
                match fs::metadata(&path) {
                    Ok(metadata) if metadata.is_file() => return Ok(path),
                    Ok(_) => {}
                    Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                    Err(source) => {
                        return Err(ResourceError::Io {
                            path,
                            message: source.to_string(),
                        });
                    }
                }
            }
        }
        Err(ResourceError::NotFound {
            kind,
            name: name.to_owned(),
            searched,
        })
    }
}

fn candidates(kind: ResourceType, name: &str) -> Result<Vec<PathBuf>, ResourceError> {
    let (directory, extensions): (&str, &[&str]) = match kind {
        ResourceType::Picture
        | ResourceType::NumberAtlas
        | ResourceType::WeatherAtlas
        | ResourceType::Mask => ("g00", &["g00", "bmp", "png", "jpg", "dds"]),
        ResourceType::Mesh => ("x", &["x"]),
        ResourceType::Sound => ("wav", &["wav", "nwa", "ogg", "owp"]),
        ResourceType::ObjectMovie => ("mov", &["omv"]),
        ResourceType::SystemMovie => ("mov", &["wmv", "mpg", "avi"]),
        ResourceType::Voice => {
            let number = name
                .parse::<u32>()
                .map_err(|_| ResourceError::InvalidName)?;
            let scene = number / 100_000;
            return Ok(vec![
                PathBuf::from(format!("koe/{scene:04}/z{number:09}.wav")),
                PathBuf::from(format!("koe/{scene:04}/z{number:09}.nwa")),
                PathBuf::from(format!("koe/z{scene:04}.ovk")),
            ]);
        }
        ResourceType::Font => ("dat", &[]),
        ResourceType::SaveThumbnail | ResourceType::Thumbnail => {
            ("", &["g00", "bmp", "png", "jpg", "dds"])
        }
        ResourceType::Other(_) => ("", &[]),
    };
    let base = Path::new(directory);
    if Path::new(name).extension().is_some() || extensions.is_empty() {
        return Ok(vec![base.join(name)]);
    }
    Ok(extensions
        .iter()
        .map(|extension| base.join(format!("{name}.{extension}")))
        .collect())
}
