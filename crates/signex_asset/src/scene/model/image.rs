use super::header::SceneHeader;
use super::types::{CommandTarget, SceneProp};

/// One checked, immutable scene inside a pack.
///
/// Holds the raw 33-field header, the code bytes, and every parsed table.
/// Table offsets (labels, Z-labels, command labels, local commands) are
/// validated to lie inside the code; string/name/namae indices are validated
/// against their tables.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneImage {
    pub(crate) header: SceneHeader,
    pub(crate) code: Vec<u8>,
    pub(crate) strings: Vec<String>,
    pub(crate) labels: Vec<u32>,
    pub(crate) z_labels: Vec<u32>,
    pub(crate) command_labels: Vec<(i32, u32)>,
    pub(crate) props: Vec<SceneProp>,
    pub(crate) prop_names: Vec<String>,
    pub(crate) commands: Vec<CommandTarget>,
    pub(crate) command_names: Vec<String>,
    pub(crate) call_prop_names: Vec<String>,
    pub(crate) namae: Vec<u32>,
    pub(crate) read_flags: Vec<i32>,
}

impl SceneImage {
    /// Raw 33-field scene header.
    pub fn header(&self) -> &SceneHeader {
        &self.header
    }

    /// Scene bytecode (`scn_ofs..scn_ofs+scn_size`).
    pub fn code(&self) -> &[u8] {
        &self.code
    }

    /// All scene strings, already XOR-decoded.
    pub fn strings(&self) -> &[String] {
        &self.strings
    }

    /// The XOR-decoded string at `index`, if present.
    pub fn string(&self, index: usize) -> Option<&str> {
        self.strings.get(index).map(String::as_str)
    }

    /// All label offsets (validated against code length).
    pub fn labels(&self) -> &[u32] {
        &self.labels
    }

    /// Label offset at `index`, if present.
    pub fn label(&self, index: usize) -> Option<u32> {
        self.labels.get(index).copied()
    }

    /// All Z-label offsets. `0` means the Z label is not defined.
    pub fn z_labels(&self) -> &[u32] {
        &self.z_labels
    }

    /// Z-label offset at `index`, if present.
    pub fn z_label(&self, index: usize) -> Option<u32> {
        self.z_labels.get(index).copied()
    }

    /// All command labels `(cmd_id, code offset)` (linker table).
    pub fn command_labels(&self) -> &[(i32, u32)] {
        &self.command_labels
    }

    /// Code offset for `cmd_id`, if present.
    pub fn command_label(&self, cmd_id: i32) -> Option<u32> {
        self.command_labels
            .iter()
            .find_map(|&(id, offset)| (id == cmd_id).then_some(offset))
    }

    /// All scene properties `{form, size}`.
    pub fn props(&self) -> &[SceneProp] {
        &self.props
    }

    /// Scene property at `index`, if present.
    pub fn prop(&self, index: usize) -> Option<SceneProp> {
        self.props.get(index).copied()
    }

    /// All scene property names (pool decoded as plain UTF-16LE).
    pub fn prop_names(&self) -> &[String] {
        &self.prop_names
    }

    /// Scene property name at `index`, if present.
    pub fn prop_name(&self, index: usize) -> Option<&str> {
        self.prop_names.get(index).map(String::as_str)
    }

    /// All local scene commands resolved to this scene's code.
    pub fn commands(&self) -> &[CommandTarget] {
        &self.commands
    }

    /// Local command target at `index`, if present.
    pub fn command(&self, index: usize) -> Option<&CommandTarget> {
        self.commands.get(index)
    }

    /// All local scene command names.
    pub fn command_names(&self) -> &[String] {
        &self.command_names
    }

    /// Local scene command name at `index`, if present.
    pub fn command_name(&self, index: usize) -> Option<&str> {
        self.command_names.get(index).map(String::as_str)
    }

    /// All call property names.
    pub fn call_prop_names(&self) -> &[String] {
        &self.call_prop_names
    }

    /// Call property name at `index`, if present.
    pub fn call_prop_name(&self, index: usize) -> Option<&str> {
        self.call_prop_names.get(index).map(String::as_str)
    }

    /// Namae list: string indices into this scene's strings.
    pub fn namae(&self) -> &[u32] {
        &self.namae
    }

    /// The resolved namae string at `index`, if present.
    pub fn namae_str(&self, index: usize) -> Option<&str> {
        let string_index = *self.namae.get(index)? as usize;
        self.strings.get(string_index).map(String::as_str)
    }

    /// All read-flag line numbers.
    pub fn read_flags(&self) -> &[i32] {
        &self.read_flags
    }

    /// Read-flag line number at `index`, if present.
    pub fn read_flag(&self, index: usize) -> Option<i32> {
        self.read_flags.get(index).copied()
    }

    pub(crate) fn code_len(&self) -> usize {
        self.code.len()
    }
}
