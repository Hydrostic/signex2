use super::header::{SceneId, ScenePackHeader};
use super::image::SceneImage;
use super::types::{CommandTarget, SceneProp};

/// A checked, immutable Scene.pck image.
///
/// Provides scene id/name lookup and include/local user-command and
/// user-property queries with the same numbering the runtime uses:
/// include entries occupy `0..inc_cmd_cnt` (resp. `inc_prop_cnt`), and
/// scene-local entries start right after.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenePack {
    pub(crate) header: ScenePackHeader,
    pub(crate) inc_props: Vec<SceneProp>,
    pub(crate) inc_prop_names: Vec<String>,
    pub(crate) inc_cmds: Vec<CommandTarget>,
    pub(crate) inc_cmd_names: Vec<String>,
    pub(crate) scn_names: Vec<String>,
    pub(crate) scenes: Vec<SceneImage>,
}

impl ScenePack {
    /// Raw 23-field pack header.
    pub fn header(&self) -> &ScenePackHeader {
        &self.header
    }

    /// Number of scenes in the pack.
    pub fn scene_count(&self) -> usize {
        self.scenes.len()
    }

    /// The scene with `id`, if present.
    pub fn scene(&self, id: SceneId) -> Option<&SceneImage> {
        usize::try_from(id.value())
            .ok()
            .and_then(|index| self.scenes.get(index))
    }

    /// Scene name at `index` (names are stored lowercase).
    pub fn scene_name(&self, index: usize) -> Option<&str> {
        self.scn_names.get(index).map(String::as_str)
    }

    /// Scene number for `name`, case-insensitive lookup.
    pub fn scene_id(&self, name: &str) -> Option<SceneId> {
        let query = name.to_lowercase();
        self.scn_names
            .iter()
            .position(|candidate| *candidate == query)
            .and_then(|index| i32::try_from(index).ok())
            .map(SceneId::new)
    }

    /// Whether the pack holds formal (compressed/encrypted) scene blocks.
    pub fn has_original_source(&self) -> bool {
        self.header.original_source_header_size > 0
    }

    /// Whether scene blocks use the EXE_EL XOR step.
    pub fn exe_angou_mode(&self) -> bool {
        self.header.exe_angou_mode != 0
    }

    /// All include (pack-level) properties.
    pub fn inc_props(&self) -> &[SceneProp] {
        &self.inc_props
    }

    /// Include property at `index`, if present.
    pub fn inc_prop(&self, index: usize) -> Option<SceneProp> {
        self.inc_props.get(index).copied()
    }

    /// All include property names.
    pub fn inc_prop_names(&self) -> &[String] {
        &self.inc_prop_names
    }

    /// Include property name at `index`, if present.
    pub fn inc_prop_name(&self, index: usize) -> Option<&str> {
        self.inc_prop_names.get(index).map(String::as_str)
    }

    /// All include (pack-level) commands.
    pub fn inc_cmds(&self) -> &[CommandTarget] {
        &self.inc_cmds
    }

    /// Include command at `index`, if present.
    pub fn inc_cmd(&self, index: usize) -> Option<&CommandTarget> {
        self.inc_cmds.get(index)
    }

    /// All include command names.
    pub fn inc_cmd_names(&self) -> &[String] {
        &self.inc_cmd_names
    }

    /// Include command name at `index`, if present.
    pub fn inc_cmd_name(&self, index: usize) -> Option<&str> {
        self.inc_cmd_names.get(index).map(String::as_str)
    }

    /// User-property number for `name` in scene `scene_id`, searching include
    /// properties first, then the scene-local properties.
    pub fn user_prop_no(&self, scene_id: SceneId, name: &str) -> Option<u32> {
        let query = name.to_lowercase();
        if let Some(index) = self
            .inc_prop_names
            .iter()
            .position(|candidate| *candidate == query)
        {
            return u32::try_from(index).ok();
        }
        let scene = self.scene(scene_id)?;
        let base = u32::try_from(self.inc_prop_names.len()).ok()?;
        let local = scene
            .prop_names()
            .iter()
            .position(|candidate| *candidate == query)?;
        base.checked_add(u32::try_from(local).ok()?)
    }

    /// Resolved property for a user-property number.
    pub fn user_prop(&self, scene_id: SceneId, user_prop_no: u32) -> Option<SceneProp> {
        let inc_len = u32::try_from(self.inc_props.len()).ok()?;
        if user_prop_no < inc_len {
            return self.inc_props.get(user_prop_no as usize).copied();
        }
        let scene = self.scene(scene_id)?;
        scene
            .props()
            .get((user_prop_no - inc_len) as usize)
            .copied()
    }

    /// Name for a user-property number.
    pub fn user_prop_name(&self, scene_id: SceneId, user_prop_no: u32) -> Option<&str> {
        let inc_len = u32::try_from(self.inc_prop_names.len()).ok()?;
        if user_prop_no < inc_len {
            return self
                .inc_prop_names
                .get(user_prop_no as usize)
                .map(String::as_str);
        }
        let scene = self.scene(scene_id)?;
        scene
            .prop_names()
            .get((user_prop_no - inc_len) as usize)
            .map(String::as_str)
    }

    /// User-command number for `name` in scene `scene_id`, searching include
    /// commands first, then the scene-local commands.
    pub fn user_cmd_no(&self, scene_id: SceneId, name: &str) -> Option<u32> {
        let query = name.to_lowercase();
        if let Some(index) = self
            .inc_cmd_names
            .iter()
            .position(|candidate| *candidate == query)
        {
            return u32::try_from(index).ok();
        }
        let scene = self.scene(scene_id)?;
        let base = u32::try_from(self.inc_cmd_names.len()).ok()?;
        let local = scene
            .command_names()
            .iter()
            .position(|candidate| *candidate == query)?;
        base.checked_add(u32::try_from(local).ok()?)
    }

    /// Resolved command target for a user-command number.
    pub fn user_cmd(&self, scene_id: SceneId, user_cmd_no: u32) -> Option<&CommandTarget> {
        let inc_len = u32::try_from(self.inc_cmds.len()).ok()?;
        if user_cmd_no < inc_len {
            return self.inc_cmds.get(user_cmd_no as usize);
        }
        let scene = self.scene(scene_id)?;
        scene.commands().get((user_cmd_no - inc_len) as usize)
    }

    /// Name for a user-command number.
    pub fn user_cmd_name(&self, scene_id: SceneId, user_cmd_no: u32) -> Option<&str> {
        let inc_len = u32::try_from(self.inc_cmd_names.len()).ok()?;
        if user_cmd_no < inc_len {
            return self
                .inc_cmd_names
                .get(user_cmd_no as usize)
                .map(String::as_str);
        }
        let scene = self.scene(scene_id)?;
        scene
            .command_names()
            .get((user_cmd_no - inc_len) as usize)
            .map(String::as_str)
    }
}
