/// A scene/user property `{form, size}` row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SceneProp {
    pub(crate) form: i32,
    pub(crate) size: i32,
}

impl SceneProp {
    /// Property form code.
    pub const fn form(&self) -> i32 {
        self.form
    }

    /// Property byte size.
    pub const fn size(&self) -> i32 {
        self.size
    }
}

/// A resolved user-command bytecode target: `(scene number, code offset)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandTarget {
    pub(crate) scene: SceneId,
    pub(crate) offset: u32,
}

impl CommandTarget {
    /// Pack scene the command lives in.
    pub const fn scene(&self) -> SceneId {
        self.scene
    }

    /// Byte offset into that scene's code (`scn_ofs`-relative).
    pub const fn offset(&self) -> u32 {
        self.offset
    }
}
use super::header::SceneId;
