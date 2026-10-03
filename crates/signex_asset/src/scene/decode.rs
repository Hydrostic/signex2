use super::error::DecodeError;
use super::keys::EASY_ANGOU;
use super::model::{
    CommandTarget, SceneHeader, SceneId, SceneImage, ScenePack, ScenePackHeader, SceneProp,
};
use super::reader::{CheckedReader, checked_count, decode_utf16};
use crate::lzss::{decode as decode_lzss, xor_cycle};

pub const SCENE_PACK_HEADER_SIZE: usize = 92;
pub const SCENE_HEADER_SIZE: usize = 132;
pub const Z_LABEL_COUNT: usize = 1000;

pub fn decode_scene(bytes: &[u8], exe_el: Option<&[u8; 16]>) -> Result<ScenePack, DecodeError> {
    let mut reader = CheckedReader::new(bytes);
    let mut raw_header = [0i32; 23];
    for field in &mut raw_header {
        *field = reader.read_i32_le()?;
    }

    let header = ScenePackHeader::from_raw(raw_header, bytes.len())?;

    let compressed = header.original_source_header_size > 0;
    let exe_mode = header.exe_angou_mode != 0;
    if compressed && exe_mode && exe_el.is_none() {
        return Err(DecodeError::KeyMissing { key: "EXE_EL" });
    }

    // --- include property list ---
    let inc_prop_count = checked_count(header.include_property_count, "include property count", 8)?;
    let inc_prop_range = section_range(
        bytes,
        header.include_property_offset,
        inc_prop_count,
        8,
        "include property list",
        4,
    )?;
    let mut inc_props = Vec::with_capacity(inc_prop_count);
    {
        let mut table =
            CheckedReader::from_slice(&bytes[inc_prop_range.clone()], u64_of(inc_prop_range.start));
        for _ in 0..inc_prop_count {
            inc_props.push(SceneProp {
                form: table.read_i32_le()?,
                size: table.read_i32_le()?,
            });
        }
    }

    let inc_prop_names = decode_name_pool(
        bytes,
        header.include_property_name_index_offset,
        header.include_property_name_index_count,
        header.include_property_name_count,
        header.include_property_name_pool_offset,
        "include property name",
        12,
    )?;
    let inc_cmd_names = decode_name_pool(
        bytes,
        header.include_command_name_index_offset,
        header.include_command_name_index_count,
        header.include_command_name_count,
        header.include_command_name_pool_offset,
        "include command name",
        36,
    )?;
    let scn_names = decode_name_pool(
        bytes,
        header.scene_name_index_offset,
        header.scene_name_index_count,
        header.scene_name_count,
        header.scene_name_pool_offset,
        "scene name",
        52,
    )?;

    // --- scene data blocks ---
    let scn_data_count = checked_count(header.scene_data_count, "scene data count", 80)?;
    if scn_data_count > scn_names.len() {
        return Err(DecodeError::InvalidRange {
            field: "scene data count",
            offset: 80,
            start: u64::try_from(scn_data_count).unwrap_or(u64::MAX),
            length: 1,
            input_len: u64::try_from(scn_names.len()).unwrap_or(u64::MAX),
        });
    }
    let data_index_range = section_range(
        bytes,
        header.scene_data_index_offset,
        scn_data_count,
        8,
        "scene data index",
        68,
    )?;
    let data_base = nonnegative(header.scene_data_offset, "scene data list offset", 76)?;
    if data_base > bytes.len() {
        return Err(DecodeError::InvalidRange {
            field: "scene data list offset",
            offset: 76,
            start: u64::try_from(data_base).unwrap_or(u64::MAX),
            length: 0,
            input_len: u64::try_from(bytes.len()).unwrap_or(u64::MAX),
        });
    }

    let mut index_reader = CheckedReader::from_slice(
        &bytes[data_index_range.clone()],
        u64_of(data_index_range.start),
    );
    let mut scene_ranges = Vec::with_capacity(scn_data_count);
    for _ in 0..scn_data_count {
        let relative = nonnegative(
            index_reader.read_i32_le()?,
            "scene data block offset",
            index_reader.offset() - 8,
        )?;
        let size = nonnegative(
            index_reader.read_i32_le()?,
            "scene data block size",
            index_reader.offset() - 4,
        )?;
        if size == 0 {
            return Err(DecodeError::InvalidRange {
                field: "scene data block size",
                offset: index_reader.offset() - 4,
                start: relative as u64,
                length: 0,
                input_len: bytes.len() as u64,
            });
        }
        let start = data_base
            .checked_add(relative)
            .ok_or(DecodeError::ArithmeticOverflow {
                field: "scene data block",
                offset: index_reader.offset() - 8,
            })?;
        let end = start
            .checked_add(size)
            .ok_or(DecodeError::ArithmeticOverflow {
                field: "scene data block",
                offset: index_reader.offset() - 4,
            })?;
        if end > bytes.len() {
            return Err(DecodeError::InvalidRange {
                field: "scene data block",
                offset: index_reader.offset() - 4,
                start: start as u64,
                length: size as u64,
                input_len: bytes.len() as u64,
            });
        }
        scene_ranges.push((start, end));
    }
    let worker_count = std::thread::available_parallelism()
        .map_or(1, |n| n.get())
        .min(4)
        .min(scn_data_count.max(1));
    let mut scenes: Vec<Option<SceneImage>> = (0..scn_data_count).map(|_| None).collect();
    std::thread::scope(|scope| {
        let mut handles = Vec::new();
        for worker in 0..worker_count {
            let ranges = &scene_ranges;
            handles.push(scope.spawn(move || {
                let mut out = Vec::new();
                let mut result: Result<(), DecodeError> = Ok(());
                for scene_no in (worker..ranges.len()).step_by(worker_count) {
                    let (start, end) = ranges[scene_no];
                    let mut data = bytes[start..end].to_vec();
                    let plain = if compressed {
                        if exe_mode && let Some(key) = exe_el {
                            xor_cycle(&mut data, key);
                        }
                        xor_cycle(&mut data, &EASY_ANGOU);
                        match decode_lzss(&data) {
                            Ok(v) => v,
                            Err(e) => {
                                result = Err(e.into());
                                break;
                            }
                        }
                    } else {
                        data
                    };
                    if plain.len() < SCENE_HEADER_SIZE {
                        result = Err(DecodeError::InvalidRange {
                            field: "scene data block",
                            offset: start as u64,
                            start: start as u64,
                            length: plain.len() as u64,
                            input_len: SCENE_HEADER_SIZE as u64,
                        });
                        break;
                    }
                    let id = match i32::try_from(scene_no) {
                        Ok(v) => SceneId::new(v),
                        Err(_) => {
                            result = Err(DecodeError::ArithmeticOverflow {
                                field: "scene number",
                                offset: 80,
                            });
                            break;
                        }
                    };
                    match decode_scene_image(&plain, id) {
                        Ok(scene) => out.push((scene_no, scene)),
                        Err(e) => {
                            result = Err(e.into());
                            break;
                        }
                    }
                }
                (out, result)
            }));
        }
        for handle in handles {
            let (out, result) = handle.join().expect("scene worker panicked");
            result?;
            for (index, scene) in out {
                scenes[index] = Some(scene);
            }
        }
        Ok::<(), DecodeError>(())
    })?;
    let scenes = scenes
        .into_iter()
        .map(|scene| scene.expect("scene worker result missing"))
        .collect::<Vec<_>>();
    // --- include command list (validated against resolved scenes) ---
    let inc_cmd_count = checked_count(header.include_command_count, "include command count", 32)?;
    let inc_cmd_range = section_range(
        bytes,
        header.include_command_offset,
        inc_cmd_count,
        8,
        "include command list",
        28,
    )?;
    let mut inc_cmds = Vec::with_capacity(inc_cmd_count);
    {
        let mut table =
            CheckedReader::from_slice(&bytes[inc_cmd_range.clone()], u64_of(inc_cmd_range.start));
        for _ in 0..inc_cmd_count {
            let scn_no = nonnegative(
                table.read_i32_le()?,
                "include command scene",
                table.offset() - 8,
            )?;
            let offset = nonnegative(
                table.read_i32_le()?,
                "include command offset",
                table.offset() - 4,
            )?;
            let target_scene = scenes.get(scn_no).ok_or(DecodeError::InvalidRange {
                field: "include command scene",
                offset: table.offset() - 8,
                start: u64::try_from(scn_no).unwrap_or(u64::MAX),
                length: 1,
                input_len: u64::try_from(scenes.len()).unwrap_or(u64::MAX),
            })?;
            let offset = checked_target(
                offset,
                target_scene.code_len(),
                "include command offset",
                table.offset() - 4,
            )?;
            inc_cmds.push(CommandTarget {
                scene: SceneId::new(i32::try_from(scn_no).map_err(|_| {
                    DecodeError::ArithmeticOverflow {
                        field: "include command scene",
                        offset: table.offset() - 8,
                    }
                })?),
                offset,
            });
        }
    }

    Ok(ScenePack {
        header,
        inc_props,
        inc_prop_names,
        inc_cmds,
        inc_cmd_names,
        scn_names,
        scenes,
    })
}

#[allow(clippy::too_many_lines)]

fn decode_scene_image(block: &[u8], scene: SceneId) -> Result<SceneImage, DecodeError> {
    let mut reader = CheckedReader::from_slice(block, 0);
    let mut raw_header = [0i32; 33];
    for field in &mut raw_header {
        *field = reader.read_i32_le()?;
    }

    let header = SceneHeader::from_raw(raw_header);
    let header_size = nonnegative(header.header_size, "scene header_size", 0)?;
    if header_size < SCENE_HEADER_SIZE || header_size > block.len() {
        return Err(DecodeError::InvalidValue {
            field: "scene header_size",
            offset: 0,
            value: i64::from(header.header_size),
        });
    }
    if header.string_index_count != header.string_count {
        return Err(DecodeError::InvalidValue {
            field: "scene string index/count",
            offset: 16,
            value: i64::from(header.string_count),
        });
    }
    for (index_cnt, name_cnt, field, offset) in [
        (
            header.property_name_index_count,
            header.property_name_count,
            "scene property name index/count",
            64,
        ),
        (
            header.command_name_index_count,
            header.command_name_count,
            "scene command name index/count",
            88,
        ),
        (
            header.call_property_name_index_count,
            header.call_property_name_count,
            "call property name index/count",
            104,
        ),
    ] {
        if index_cnt != name_cnt {
            return Err(DecodeError::InvalidValue {
                field,
                offset,
                value: i64::from(name_cnt),
            });
        }
    }

    // --- code range ---
    let code_ofs = nonnegative(header.code_offset, "scene code offset", 4)?;
    let code_size = nonnegative(header.code_size, "scene code size", 8)?;
    let code_end = code_ofs
        .checked_add(code_size)
        .ok_or(DecodeError::ArithmeticOverflow {
            field: "scene code range",
            offset: 4,
        })?;
    if code_end > block.len() {
        return Err(DecodeError::InvalidRange {
            field: "scene code",
            offset: 4,
            start: u64::try_from(code_ofs).unwrap_or(u64::MAX),
            length: u64::try_from(code_size).unwrap_or(u64::MAX),
            input_len: u64::try_from(block.len()).unwrap_or(u64::MAX),
        });
    }
    let code = block[code_ofs..code_end].to_vec();

    // --- strings (XOR-decoded) ---
    let strings = decode_strings(
        block,
        header.string_index_offset,
        header.string_index_count,
        header.string_pool_offset,
        "scene string",
        12,
    )?;

    // --- labels / z-labels ---
    let labels = decode_offset_table(
        block,
        header.label_offset,
        header.label_count,
        code.len(),
        "scene label",
        28,
    )?;
    let _z_label_count = checked_count(header.z_label_count, "scene z-label count", 40)?;
    let z_labels = decode_offset_table(
        block,
        header.z_label_offset,
        header.z_label_count,
        code.len(),
        "scene z-label",
        36,
    )?;

    // --- command labels {cmd_id, offset} ---
    let command_labels = decode_command_labels(
        block,
        header.command_label_offset,
        header.command_label_count,
        code.len(),
        "scene command label",
        44,
    )?;

    // --- properties ---
    let prop_count = checked_count(header.property_count, "scene property count", 56)?;
    let prop_range = section_range(
        block,
        header.property_offset,
        prop_count,
        8,
        "scene property list",
        52,
    )?;
    let mut props = Vec::with_capacity(prop_count);
    {
        let mut table =
            CheckedReader::from_slice(&block[prop_range.clone()], u64_of(prop_range.start));
        for _ in 0..prop_count {
            props.push(SceneProp {
                form: table.read_i32_le()?,
                size: table.read_i32_le()?,
            });
        }
    }
    let prop_names = decode_name_pool(
        block,
        header.property_name_index_offset,
        header.property_name_index_count,
        header.property_name_count,
        header.property_name_pool_offset,
        "scene property name",
        60,
    )?;
    if props.len() != prop_names.len() {
        return Err(DecodeError::InvalidValue {
            field: "scene property count/name",
            offset: 56,
            value: i64::try_from(props.len()).unwrap_or(i64::MAX),
        });
    }

    // --- local commands {offset} ---
    let command_count = checked_count(header.command_count, "scene command count", 80)?;
    let command_range = section_range(
        block,
        header.command_offset,
        command_count,
        4,
        "scene command list",
        76,
    )?;
    let mut commands = Vec::with_capacity(command_count);
    {
        let mut table =
            CheckedReader::from_slice(&block[command_range.clone()], u64_of(command_range.start));
        for _ in 0..command_count {
            let offset = table.read_i32_le()?;
            commands.push(CommandTarget {
                scene,
                offset: checked_target(
                    nonnegative(offset, "scene command offset", table.offset() - 4)?,
                    code.len(),
                    "scene command offset",
                    table.offset() - 4,
                )?,
            });
        }
    }
    let command_names = decode_name_pool(
        block,
        header.command_name_index_offset,
        header.command_name_index_count,
        header.command_name_count,
        header.command_name_pool_offset,
        "scene command name",
        84,
    )?;
    if commands.len() != command_names.len() {
        return Err(DecodeError::InvalidValue {
            field: "scene command count/name",
            offset: 80,
            value: i64::try_from(commands.len()).unwrap_or(i64::MAX),
        });
    }
    let call_prop_names = decode_name_pool(
        block,
        header.call_property_name_index_offset,
        header.call_property_name_index_count,
        header.call_property_name_count,
        header.call_property_name_pool_offset,
        "call property name",
        100,
    )?;

    // --- namae (string indices) ---
    let namae_values = read_i32_table(
        block,
        header.namae_offset,
        header.namae_count,
        "scene namae",
        116,
    )?;
    let mut namae = Vec::with_capacity(namae_values.len());
    for value in namae_values {
        let index = nonnegative(value, "scene namae string index", 116)?;
        if index >= strings.len() {
            return Err(DecodeError::InvalidRange {
                field: "scene namae string index",
                offset: 116,
                start: u64::try_from(index).unwrap_or(u64::MAX),
                length: 1,
                input_len: u64::try_from(strings.len()).unwrap_or(u64::MAX),
            });
        }
        namae.push(
            u32::try_from(index).map_err(|_| DecodeError::ArithmeticOverflow {
                field: "scene namae string index",
                offset: 116,
            })?,
        );
    }

    // --- read flags (line numbers) ---
    let read_flags = read_i32_table(
        block,
        header.read_flag_offset,
        header.read_flag_count,
        "scene read flag",
        124,
    )?;

    Ok(SceneImage {
        header,

        code,
        strings,
        labels,
        z_labels,
        command_labels,
        props,
        prop_names,
        commands,
        command_names,
        call_prop_names,
        namae,
        read_flags,
    })
}

#[allow(clippy::too_many_arguments)]
fn decode_name_pool(
    bytes: &[u8],
    index_ofs: i32,
    index_cnt: i32,
    name_cnt: i32,
    pool_ofs: i32,
    field: &'static str,
    offset: u64,
) -> Result<Vec<String>, DecodeError> {
    if index_cnt != name_cnt {
        return Err(DecodeError::InvalidValue {
            field,
            offset,
            value: i64::from(name_cnt),
        });
    }
    let count = checked_count(index_cnt, field, offset)?;
    let range = section_range(bytes, index_ofs, count, 8, field, offset)?;
    let pool_start = nonnegative(pool_ofs, field, offset)?;
    if pool_start > bytes.len() {
        return Err(DecodeError::InvalidRange {
            field,
            offset,
            start: u64::try_from(pool_start).unwrap_or(u64::MAX),
            length: 0,
            input_len: u64::try_from(bytes.len()).unwrap_or(u64::MAX),
        });
    }
    let mut index_reader = CheckedReader::from_slice(&bytes[range.clone()], u64_of(range.start));
    let mut names = Vec::with_capacity(count);
    for _ in 0..count {
        let units_ofs = index_reader.read_i32_le()?;
        let units_size = index_reader.read_i32_le()?;
        let byte_start = pool_range(
            bytes.len(),
            pool_start,
            units_ofs,
            units_size,
            field,
            index_reader.offset() - 8,
        )?;
        names.push(decode_utf16(
            &bytes[byte_start.start..byte_start.end],
            field,
            u64_of(byte_start.start),
        )?);
    }
    Ok(names)
}

fn decode_strings(
    block: &[u8],
    index_ofs: i32,
    index_cnt: i32,
    pool_ofs: i32,
    field: &'static str,
    offset: u64,
) -> Result<Vec<String>, DecodeError> {
    let count = checked_count(index_cnt, field, offset)?;
    let range = section_range(block, index_ofs, count, 8, field, offset)?;
    let pool_start = nonnegative(pool_ofs, field, offset)?;
    if pool_start > block.len() {
        return Err(DecodeError::InvalidRange {
            field,
            offset,
            start: u64::try_from(pool_start).unwrap_or(u64::MAX),
            length: 0,
            input_len: u64::try_from(block.len()).unwrap_or(u64::MAX),
        });
    }
    let mut index_reader = CheckedReader::from_slice(&block[range.clone()], u64_of(range.start));
    let mut strings = Vec::with_capacity(count);
    for string_no in 0..count {
        let units_ofs = index_reader.read_i32_le()?;
        let units_size = index_reader.read_i32_le()?;
        let byte_range = pool_range(
            block.len(),
            pool_start,
            units_ofs,
            units_size,
            field,
            index_reader.offset() - 8,
        )?;
        strings.push(decode_xor_utf16(
            &block[byte_range.start..byte_range.end],
            string_no,
            field,
            u64_of(byte_range.start),
        )?);
    }
    Ok(strings)
}

/// Computes and validates the byte range `[pool_start + offset*2,
/// pool_start + (offset+size)*2)` for one UTF-16 name/string slice.
fn pool_range(
    buffer_len: usize,
    pool_start: usize,
    units_ofs: i32,
    units_size: i32,
    field: &'static str,
    offset: u64,
) -> Result<std::ops::Range<usize>, DecodeError> {
    let units_ofs = nonnegative(units_ofs, field, offset)?;
    let units_size = nonnegative(units_size, field, offset)?;
    let start = units_ofs
        .checked_mul(2)
        .ok_or(DecodeError::ArithmeticOverflow { field, offset })?;
    let length = units_size
        .checked_mul(2)
        .ok_or(DecodeError::ArithmeticOverflow { field, offset })?;
    let end = start
        .checked_add(length)
        .ok_or(DecodeError::ArithmeticOverflow { field, offset })?;
    let byte_start = pool_start
        .checked_add(start)
        .ok_or(DecodeError::ArithmeticOverflow { field, offset })?;
    let byte_end = pool_start
        .checked_add(end)
        .ok_or(DecodeError::ArithmeticOverflow { field, offset })?;
    if byte_end > buffer_len {
        return Err(DecodeError::InvalidRange {
            field,
            offset,
            start: u64::try_from(byte_start).unwrap_or(u64::MAX),
            length: u64::try_from(length).unwrap_or(u64::MAX),
            input_len: u64::try_from(buffer_len).unwrap_or(u64::MAX),
        });
    }
    Ok(byte_start..byte_end)
}

/// Decodes one XORed scene string: UTF-16LE code units XORed with
/// `(28807 * string_index) & 0xffff`.
fn decode_xor_utf16(
    bytes: &[u8],
    string_index: usize,
    field: &'static str,
    offset: u64,
) -> Result<String, DecodeError> {
    if !bytes.len().is_multiple_of(2) {
        return Err(DecodeError::InvalidValue {
            field,
            offset,
            value: i64::try_from(bytes.len()).unwrap_or(i64::MAX),
        });
    }
    let key = (28_807u32.wrapping_mul(string_index as u32) & 0xffff) as u16;
    let units = bytes
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]) ^ key)
        .collect::<Vec<_>>();
    String::from_utf16(&units).map_err(|_| DecodeError::InvalidUtf16 { field, offset })
}

fn read_i32_table(
    bytes: &[u8],
    ofs: i32,
    cnt: i32,
    field: &'static str,
    offset: u64,
) -> Result<Vec<i32>, DecodeError> {
    let count = checked_count(cnt, field, offset)?;
    let range = section_range(bytes, ofs, count, 4, field, offset)?;
    let mut table = CheckedReader::from_slice(&bytes[range.clone()], u64_of(range.start));
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        out.push(table.read_i32_le()?);
    }
    Ok(out)
}

fn decode_offset_table(
    block: &[u8],
    ofs: i32,
    cnt: i32,
    max: usize,
    field: &'static str,
    offset: u64,
) -> Result<Vec<u32>, DecodeError> {
    let values = read_i32_table(block, ofs, cnt, field, offset)?;
    values
        .into_iter()
        .map(|value| checked_target(nonnegative(value, field, offset)?, max, field, offset))
        .collect()
}

fn decode_command_labels(
    block: &[u8],
    ofs: i32,
    cnt: i32,
    max: usize,
    field: &'static str,
    offset: u64,
) -> Result<Vec<(i32, u32)>, DecodeError> {
    let count = checked_count(cnt, field, offset)?;
    let range = section_range(block, ofs, count, 8, field, offset)?;
    let mut table = CheckedReader::from_slice(&block[range.clone()], u64_of(range.start));
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        let cmd_id = table.read_i32_le()?;
        let target = table.read_i32_le()?;
        out.push((
            cmd_id,
            checked_target(
                nonnegative(target, field, table.offset() - 4)?,
                max,
                field,
                table.offset() - 4,
            )?,
        ));
    }
    Ok(out)
}

/// Validates that a table section `[ofs, ofs + count*item_size)` lies inside
/// `bytes`. `count` must already be a checked, non-negative value.
fn section_range(
    bytes: &[u8],
    ofs: i32,
    count: usize,
    item_size: usize,
    field: &'static str,
    offset: u64,
) -> Result<std::ops::Range<usize>, DecodeError> {
    let start = nonnegative(ofs, field, offset)?;
    let length = count
        .checked_mul(item_size)
        .ok_or(DecodeError::ArithmeticOverflow { field, offset })?;
    let end = start
        .checked_add(length)
        .ok_or(DecodeError::ArithmeticOverflow { field, offset })?;
    if end > bytes.len() {
        return Err(DecodeError::InvalidRange {
            field,
            offset,
            start: u64::try_from(start).unwrap_or(u64::MAX),
            length: u64::try_from(length).unwrap_or(u64::MAX),
            input_len: u64::try_from(bytes.len()).unwrap_or(u64::MAX),
        });
    }
    Ok(start..end)
}

fn nonnegative(value: i32, field: &'static str, offset: u64) -> Result<usize, DecodeError> {
    usize::try_from(value).map_err(|_| DecodeError::InvalidValue {
        field,
        offset,
        value: i64::from(value),
    })
}

fn checked_target(
    value: usize,
    max: usize,
    field: &'static str,
    offset: u64,
) -> Result<u32, DecodeError> {
    if value > max {
        return Err(DecodeError::InvalidRange {
            field,
            offset,
            start: u64::try_from(value).unwrap_or(u64::MAX),
            length: 1,
            input_len: u64::try_from(max).unwrap_or(u64::MAX),
        });
    }
    u32::try_from(value).map_err(|_| DecodeError::ArithmeticOverflow { field, offset })
}

fn u64_of(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}
