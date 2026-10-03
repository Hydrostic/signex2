/// Stable scene number used by ScenePack lookups.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SceneId(i32);

impl SceneId {
    pub const fn new(value: i32) -> Self {
        Self(value)
    }
    pub const fn value(self) -> i32 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenePackHeader {
    pub header_size: i32,
    pub include_property_offset: i32,
    pub include_property_count: i32,
    pub include_property_name_index_offset: i32,
    pub include_property_name_index_count: i32,
    pub include_property_name_pool_offset: i32,
    pub include_property_name_count: i32,
    pub include_command_offset: i32,
    pub include_command_count: i32,
    pub include_command_name_index_offset: i32,
    pub include_command_name_index_count: i32,
    pub include_command_name_pool_offset: i32,
    pub include_command_name_count: i32,
    pub scene_name_index_offset: i32,
    pub scene_name_index_count: i32,
    pub scene_name_pool_offset: i32,
    pub scene_name_count: i32,
    pub scene_data_index_offset: i32,
    pub scene_data_index_count: i32,
    pub scene_data_offset: i32,
    pub scene_data_count: i32,
    pub exe_angou_mode: i32,
    pub original_source_header_size: i32,
    raw: [i32; 23],
}

impl ScenePackHeader {
    pub(crate) fn from_raw(
        raw: [i32; 23],
        input_len: usize,
    ) -> Result<Self, super::super::error::DecodeError> {
        let header_size = usize::try_from(raw[0]).map_err(|_| {
            super::super::error::DecodeError::InvalidValue {
                field: "Scene pack header_size",
                offset: 0,
                value: i64::from(raw[0]),
            }
        })?;
        if header_size < 92 || header_size > input_len {
            return Err(super::super::error::DecodeError::InvalidValue {
                field: "Scene pack header_size",
                offset: 0,
                value: i64::from(raw[0]),
            });
        }
        if raw[22] < 0 {
            return Err(super::super::error::DecodeError::InvalidValue {
                field: "Scene original_source_header_size",
                offset: 88,
                value: i64::from(raw[22]),
            });
        }
        for (index_count, name_count, field, offset) in [
            (raw[4], raw[6], "include property name index/count", 16),
            (raw[10], raw[12], "include command name index/count", 40),
            (raw[14], raw[16], "scene name index/count", 56),
            (raw[18], raw[20], "scene data index/count", 72),
        ] {
            if index_count != name_count {
                return Err(super::super::error::DecodeError::InvalidValue {
                    field,
                    offset,
                    value: i64::from(name_count),
                });
            }
        }
        Ok(Self {
            header_size: raw[0],
            include_property_offset: raw[1],
            include_property_count: raw[2],
            include_property_name_index_offset: raw[3],
            include_property_name_index_count: raw[4],
            include_property_name_pool_offset: raw[5],
            include_property_name_count: raw[6],
            include_command_offset: raw[7],
            include_command_count: raw[8],
            include_command_name_index_offset: raw[9],
            include_command_name_index_count: raw[10],
            include_command_name_pool_offset: raw[11],
            include_command_name_count: raw[12],
            scene_name_index_offset: raw[13],
            scene_name_index_count: raw[14],
            scene_name_pool_offset: raw[15],
            scene_name_count: raw[16],
            scene_data_index_offset: raw[17],
            scene_data_index_count: raw[18],
            scene_data_offset: raw[19],
            scene_data_count: raw[20],
            exe_angou_mode: raw[21],
            original_source_header_size: raw[22],
            raw,
        })
    }
    pub fn raw_fields(&self) -> &[i32; 23] {
        &self.raw
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneHeader {
    pub header_size: i32,
    pub code_offset: i32,
    pub code_size: i32,
    pub string_index_offset: i32,
    pub string_index_count: i32,
    pub string_pool_offset: i32,
    pub string_count: i32,
    pub label_offset: i32,
    pub label_count: i32,
    pub z_label_offset: i32,
    pub z_label_count: i32,
    pub command_label_offset: i32,
    pub command_label_count: i32,
    pub property_offset: i32,
    pub property_count: i32,
    pub property_name_index_offset: i32,
    pub property_name_index_count: i32,
    pub property_name_pool_offset: i32,
    pub property_name_count: i32,
    pub command_offset: i32,
    pub command_count: i32,
    pub command_name_index_offset: i32,
    pub command_name_index_count: i32,
    pub command_name_pool_offset: i32,
    pub command_name_count: i32,
    pub call_property_name_index_offset: i32,
    pub call_property_name_index_count: i32,
    pub call_property_name_pool_offset: i32,
    pub call_property_name_count: i32,
    pub namae_offset: i32,
    pub namae_count: i32,
    pub read_flag_offset: i32,
    pub read_flag_count: i32,
    raw: [i32; 33],
}

impl SceneHeader {
    pub(crate) fn from_raw(raw: [i32; 33]) -> Self {
        Self {
            header_size: raw[0],
            code_offset: raw[1],
            code_size: raw[2],
            string_index_offset: raw[3],
            string_index_count: raw[4],
            string_pool_offset: raw[5],
            string_count: raw[6],
            label_offset: raw[7],
            label_count: raw[8],
            z_label_offset: raw[9],
            z_label_count: raw[10],
            command_label_offset: raw[11],
            command_label_count: raw[12],
            property_offset: raw[13],
            property_count: raw[14],
            property_name_index_offset: raw[15],
            property_name_index_count: raw[16],
            property_name_pool_offset: raw[17],
            property_name_count: raw[18],
            command_offset: raw[19],
            command_count: raw[20],
            command_name_index_offset: raw[21],
            command_name_index_count: raw[22],
            command_name_pool_offset: raw[23],
            command_name_count: raw[24],
            call_property_name_index_offset: raw[25],
            call_property_name_index_count: raw[26],
            call_property_name_pool_offset: raw[27],
            call_property_name_count: raw[28],
            namae_offset: raw[29],
            namae_count: raw[30],
            read_flag_offset: raw[31],
            read_flag_count: raw[32],
            raw,
        }
    }
    pub fn raw_fields(&self) -> &[i32; 33] {
        &self.raw
    }
}
