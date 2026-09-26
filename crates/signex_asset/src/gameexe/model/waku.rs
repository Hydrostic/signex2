//! Message window frames, buttons, and face positions.

use super::helpers::{OneToThree, checked_count, one_to_three};
use crate::gameexe::{
    GArray, GameexeError, GameexeNode, GameexeValue, Span, SpannedToken, Token, gameexe,
};

#[derive(Debug)]
pub struct WakuConfig {
    pub frames: GArray<WakuFrame>,
    pub button_count: usize,
    pub face_count: usize,
    pub object_count: usize,
}

impl GameexeNode for WakuConfig {
    fn gameexe_default() -> Self {
        Self {
            frames: GArray::new(4, 0, 256, |_| WakuFrame::gameexe_default()),
            button_count: 8,
            face_count: 1,
            object_count: 1,
        }
    }

    fn apply(
        &mut self,
        path: &[&str],
        value: &[SpannedToken<'_>],
        span: Span,
    ) -> Result<(), GameexeError> {
        match path {
            [field] if field.eq_ignore_ascii_case("CNT") => {
                self.frames
                    .set_count(i32::parse_value(value, "WAKU.CNT")?, span, "WAKU.CNT")
            }
            [group, field]
                if group.eq_ignore_ascii_case("BTN") && field.eq_ignore_ascii_case("CNT") =>
            {
                self.set_button_count(value, span)
            }
            [group, field]
                if group.eq_ignore_ascii_case("FACE") && field.eq_ignore_ascii_case("CNT") =>
            {
                self.set_face_count(value, span)
            }
            [group, field]
                if group.eq_ignore_ascii_case("OBJECT") && field.eq_ignore_ascii_case("CNT") =>
            {
                self.object_count = checked_count(value, 0, 16, "WAKU.OBJECT.CNT")?;
                Ok(())
            }
            [index, rest @ ..]
                if !rest.is_empty() && index.bytes().all(|byte| byte.is_ascii_digit()) =>
            {
                let index: usize = index
                    .parse()
                    .map_err(|_| GameexeError::new(span.clone(), *index, "array index overflow"))?;
                let Some(frame) = self.frames.as_mut_slice().get_mut(index) else {
                    return Err(GameexeError::new(
                        span,
                        index.to_string(),
                        "frame index outside WAKU.CNT",
                    ));
                };
                frame
                    .btn
                    .set_count(self.button_count as i32, span.clone(), "WAKU.BTN.CNT")?;
                frame
                    .face
                    .set_count(self.face_count as i32, span.clone(), "WAKU.FACE.CNT")?;
                frame.apply(rest, value, span)
            }
            _ => Err(GameexeError::new(span, "WAKU", "unknown WAKU field")),
        }
    }
}

impl WakuConfig {
    fn set_button_count(
        &mut self,
        value: &[SpannedToken<'_>],
        span: Span,
    ) -> Result<(), GameexeError> {
        let count = checked_count(value, 0, 256, "WAKU.BTN.CNT")?;
        self.button_count = count;
        for frame in self.frames.as_mut_slice() {
            frame
                .btn
                .set_count(count as i32, span.clone(), "WAKU.BTN.CNT")?;
        }
        Ok(())
    }

    fn set_face_count(
        &mut self,
        value: &[SpannedToken<'_>],
        span: Span,
    ) -> Result<(), GameexeError> {
        let count = checked_count(value, 0, 16, "WAKU.FACE.CNT")?;
        self.face_count = count;
        for frame in self.frames.as_mut_slice() {
            frame
                .face
                .set_count(count as i32, span.clone(), "WAKU.FACE.CNT")?;
        }
        Ok(())
    }
}



#[gameexe]
#[derive(Debug)]
pub struct WakuFrame {
    pub extend_type: i32,
    pub waku_file: String,
    pub filter_file: String,
    pub filter_margin: (i32, i32, i32, i32),
    #[gameexe(default = (0, 0, 255, 128))]
    pub filter_color: (i32, i32, i32, i32),
    #[gameexe(default = true)]
    pub filter_config_color: bool,
    #[gameexe(default = true)]
    pub filter_config_tr: bool,
    #[gameexe(default = -1)]
    pub icon_no: i32,
    #[gameexe(default = -1)]
    pub page_icon_no: i32,
    pub icon_pos_type: i32,
    pub icon_pos: (i32, i32, i32),
    #[gameexe(array(count = "", default_count = 8, count_min = 0, count_max = 256))]
    pub btn: GArray<WakuButton>,
    #[gameexe(array(count = "", default_count = 1, count_min = 0, count_max = 16))]
    pub face: GArray<WakuFace>,
}

#[gameexe]
#[derive(Debug)]
pub struct WakuButton {
    pub file: String,
    pub cut_no: i32,
    pub pos: (i32, i32, i32),
    pub action: i32,
    pub se: i32,
    #[gameexe(name = "TYPE", value, default = WakuButtonType::None)]
    pub r#type: WakuButtonType,
    #[gameexe(value, default = WakuCall::None)]
    pub call: WakuCall,
    pub frame_action: (String, String),
}

#[gameexe]
#[derive(Debug)]
pub struct WakuFace {
    pub pos: (i32, i32),
}

#[derive(Debug)]
pub enum WakuButtonType {
    None,
    Simple(String),
    WithOption(String, i32),
    WithMode(String, i32, i32),
}

impl GameexeValue for WakuButtonType {
    fn parse_value(value: &[SpannedToken<'_>], path: &str) -> Result<Self, GameexeError> {
        let parts = one_to_three(value, path)?;
        let (name, option, mode) = match parts {
            OneToThree::One(name) => (String::parse_value(name, path)?, None, None),
            OneToThree::Two(name, option) => (
                String::parse_value(name, path)?,
                Some(i32::parse_value(option, path)?),
                None,
            ),
            OneToThree::Three(name, option, mode) => (
                String::parse_value(name, path)?,
                Some(i32::parse_value(option, path)?),
                Some(i32::parse_value(mode, path)?),
            ),
        };
        match (name.as_str(), option, mode) {
            ("none", None, None) => Ok(Self::None),
            (
                "save" | "load" | "return_sel" | "close_mwnd" | "msg_log" | "koe_play" | "config",
                None,
                None,
            ) => Ok(Self::Simple(name)),
            ("qsave" | "qload" | "read_skip" | "auto_mode", Some(option), None) => {
                Ok(Self::WithOption(name, option))
            }
            (
                "local_switch" | "global_switch" | "local_mode" | "global_mode",
                Some(option),
                Some(mode),
            ) => Ok(Self::WithMode(name, option, mode)),
            _ => Err(GameexeError::new(
                value.first().map_or(0..0, |token| token.span.clone()),
                path,
                "unsupported button type",
            )),
        }
    }
}

#[derive(Debug)]
pub enum WakuCall {
    None,
    Command(String, String),
    Label(String, i32),
}

impl GameexeValue for WakuCall {
    fn parse_value(value: &[SpannedToken<'_>], path: &str) -> Result<Self, GameexeError> {
        let OneToThree::Two(first, second) = one_to_three(value, path)? else {
            return Err(GameexeError::new(
                value.first().map_or(0..0, |token| token.span.clone()),
                path,
                "expected call target",
            ));
        };
        let name = String::parse_value(first, path)?;
        let token = second.first().ok_or_else(|| {
            GameexeError::new(
                value.first().map_or(0..0, |token| token.span.clone()),
                path,
                "expected call target",
            )
        })?;
        match &token.token {
            Token::String(_) => Ok(Self::Command(name, String::parse_value(second, path)?)),
            Token::Integer(_) => Ok(Self::Label(name, i32::parse_value(second, path)?)),
            _ => Err(GameexeError::new(
                token.span.clone(),
                path,
                "invalid call target",
            )),
        }
    }
}
