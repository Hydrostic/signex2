//! Checked, ordered application of Gameexe.ini entries.

use super::{GArray, Lexer, Span, SpannedToken, Token};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{path} at byte {} ({}): {message}", .span.start, .span.end)]
pub struct GameexeError {
    pub span: Span,
    pub path: String,
    pub message: String,
}

impl GameexeError {
    pub fn new(span: Span, path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            span,
            path: path.into(),
            message: message.into(),
        }
    }
}

pub trait GameexeNode: Sized {
    fn gameexe_default() -> Self;
    fn apply(
        &mut self,
        path: &[&str],
        value: &[SpannedToken<'_>],
        span: Span,
    ) -> Result<(), GameexeError>;
}

pub trait GameexeValue: Sized {
    fn parse_value(value: &[SpannedToken<'_>], path: &str) -> Result<Self, GameexeError>;

    fn update_value(&mut self, value: &[SpannedToken<'_>], path: &str) -> Result<(), GameexeError> {
        *self = Self::parse_value(value, path)?;
        Ok(())
    }
}

fn span_of_tokens(value: &[SpannedToken<'_>]) -> Span {
    match (value.first(), value.last()) {
        (Some(first), Some(last)) => first.span.start..last.span.end,
        _ => 0..0,
    }
}

fn expect_single_token<'a>(
    value: &'a [SpannedToken<'a>],
    path: &str,
) -> Result<&'a SpannedToken<'a>, GameexeError> {
    if value.len() == 1 {
        Ok(&value[0])
    } else {
        Err(GameexeError::new(
            span_of_tokens(value),
            path,
            "expected one value",
        ))
    }
}

impl GameexeValue for i32 {
    fn parse_value(value: &[SpannedToken<'_>], path: &str) -> Result<Self, GameexeError> {
        let token = expect_single_token(value, path)?;
        if let Token::Integer(raw) = &token.token {
            raw.parse().map_err(|_| {
                GameexeError::new(token.span.clone(), path, "integer outside i32 range")
            })
        } else {
            Err(GameexeError::new(
                token.span.clone(),
                path,
                "expected integer",
            ))
        }
    }
}

impl GameexeValue for u32 {
    fn parse_value(value: &[SpannedToken<'_>], path: &str) -> Result<Self, GameexeError> {
        let token = expect_single_token(value, path)?;
        if let Token::Integer(raw) = &token.token {
            raw.parse().map_err(|_| {
                GameexeError::new(token.span.clone(), path, "integer outside u32 range")
            })
        } else {
            Err(GameexeError::new(
                token.span.clone(),
                path,
                "expected integer",
            ))
        }
    }
}

impl GameexeValue for bool {
    fn parse_value(value: &[SpannedToken<'_>], path: &str) -> Result<Self, GameexeError> {
        Ok(i32::parse_value(value, path)? != 0)
    }
}

impl GameexeValue for String {
    fn parse_value(value: &[SpannedToken<'_>], path: &str) -> Result<Self, GameexeError> {
        let token = expect_single_token(value, path)?;
        let Token::String(raw) = &token.token else {
            return Err(GameexeError::new(
                token.span.clone(),
                path,
                "expected quoted string",
            ));
        };
        let mut decoded = String::with_capacity(raw.len());
        let mut chars = raw.chars();
        while let Some(ch) = chars.next() {
            if ch == '\\' {
                match chars.next() {
                    Some('\\') => decoded.push('\\'),
                    Some('"') => decoded.push('"'),
                    _ => {
                        return Err(GameexeError::new(
                            token.span.clone(),
                            path,
                            "invalid string escape",
                        ));
                    }
                }
            } else {
                decoded.push(ch);
            }
        }
        Ok(decoded)
    }
}

macro_rules! tuple_value {
    ($($ty:ident:$idx:tt),+) => {
        impl<$($ty: GameexeValue),+> GameexeValue for ($($ty,)+) {
            fn parse_value(value: &[SpannedToken<'_>], path: &str) -> Result<Self, GameexeError> {
                let mut parts = Vec::new();
                let mut start = 0;
                for (index, token) in value.iter().enumerate() {
                    if token.token == Token::Comma {
                        if index == start { return Err(GameexeError::new(token.span.clone(), path, "empty tuple element")); }
                        parts.push(&value[start..index]);
                        start = index + 1;
                    }
                }
                if start == value.len() { return Err(GameexeError::new(span_of_tokens(value), path, "empty tuple element")); }
                parts.push(&value[start..]);
                if parts.len() != [$(stringify!($ty)),+].len() {
                    return Err(GameexeError::new(span_of_tokens(value), path, "wrong tuple arity"));
                }
                Ok(($($ty::parse_value(parts[$idx], path)?,)+))
            }
        }
    };
}
tuple_value!(A:0, B:1);
tuple_value!(A:0, B:1, C:2);
tuple_value!(A:0, B:1, C:2, D:3);
tuple_value!(A:0, B:1, C:2, D:3, E:4);
tuple_value!(A:0, B:1, C:2, D:3, E:4, F:5);

impl<T: GameexeValue> GameexeValue for Vec<T> {
    fn parse_value(value: &[SpannedToken<'_>], path: &str) -> Result<Self, GameexeError> {
        let mut result = Vec::new();
        let mut index = 0;
        while index < value.len() {
            if value[index].token != Token::LeftParen {
                return Err(GameexeError::new(
                    value[index].span.clone(),
                    path,
                    "expected opening parenthesis",
                ));
            }
            let start = index + 1;
            index = start;
            while index < value.len() && value[index].token != Token::RightParen {
                index += 1;
            }
            if index == value.len() {
                return Err(GameexeError::new(
                    span_of_tokens(value),
                    path,
                    "missing closing parenthesis",
                ));
            }
            result.push(T::parse_value(&value[start..index], path)?);
            index += 1;
        }
        if result.is_empty() {
            return Err(GameexeError::new(
                span_of_tokens(value),
                path,
                "expected at least one group",
            ));
        }
        Ok(result)
    }

    fn update_value(&mut self, value: &[SpannedToken<'_>], path: &str) -> Result<(), GameexeError> {
        self.extend(Self::parse_value(value, path)?);
        Ok(())
    }
}

impl<T> GArray<T> {
    pub fn set_count(&mut self, count: i32, span: Span, path: &str) -> Result<(), GameexeError> {
        let (min_count, max_count) = self.count_bounds();
        if count < 0 || (count as usize) < min_count || (count as usize) > max_count {
            return Err(GameexeError::new(
                span,
                path,
                format!("count must be {min_count}..={max_count}"),
            ));
        }
        self.set_active_count(count as usize);
        Ok(())
    }
}

// A path such as OBJECT.000-099.USE updates a field on each existing item.
// Each item parses the same tokens independently, so T does not need Clone.
impl<T: GameexeNode> GArray<T> {
    pub fn apply(
        &mut self,
        path: &[&str],
        value: &[SpannedToken<'_>],
        span: Span,
    ) -> Result<(), GameexeError> {
        let Some((&head, tail)) = path.split_first() else {
            return Err(GameexeError::new(span, "", "expected array index or CNT"));
        };
        if head.eq_ignore_ascii_case("CNT") {
            if !tail.is_empty() {
                return Err(GameexeError::new(span, head, "unexpected path after CNT"));
            }
            return self.set_count(i32::parse_value(value, head)?, span, head);
        }
        let (first, last) = if let Some((a, b)) = head.split_once('-') {
            (a, b)
        } else {
            (head, head)
        };
        let parse_index = |raw: &str| -> Result<usize, GameexeError> {
            if raw.is_empty() || !raw.bytes().all(|b| b.is_ascii_digit()) {
                return Err(GameexeError::new(
                    span.clone(),
                    head,
                    "expected decimal array index",
                ));
            }
            let index: usize = raw
                .parse()
                .map_err(|_| GameexeError::new(span.clone(), head, "array index overflow"))?;
            if index >= self.len() {
                return Err(GameexeError::new(
                    span.clone(),
                    head,
                    format!("index {index} outside 0..{}", self.len()),
                ));
            }
            Ok(index)
        };
        let first = parse_index(first)?;
        let last = parse_index(last)?;
        if first > last {
            return Ok(());
        }
        for item in &mut self.as_mut_slice()[first..=last] {
            item.apply(tail, value, span.clone())?;
        }
        Ok(())
    }

    /// Apply an index against the fixed storage capacity, even if it is above CNT.
    /// The original FLICK_SCENE branch does not check the active count.
    pub fn apply_capacity(
        &mut self,
        path: &[&str],
        value: &[SpannedToken<'_>],
        span: Span,
    ) -> Result<(), GameexeError> {
        let Some((&head, tail)) = path.split_first() else {
            return Err(GameexeError::new(span, "", "expected array index"));
        };
        if !head.bytes().all(|byte| byte.is_ascii_digit()) || head.is_empty() {
            return Err(GameexeError::new(
                span,
                head,
                "expected decimal array index",
            ));
        }
        let index: usize = head
            .parse()
            .map_err(|_| GameexeError::new(span.clone(), head, "array index overflow"))?;
        let item = self
            .storage_mut(index)
            .ok_or_else(|| GameexeError::new(span.clone(), head, "array index outside capacity"))?;
        item.apply(tail, value, span)
    }
}

// A path such as SHAKE.000 assigns a complete value to an indexed item.
// Values decide whether assignment replaces or appends (SHAKE appends).
impl<T: GameexeValue> GArray<T> {
    pub fn apply_value(
        &mut self,
        path: &[&str],
        value: &[SpannedToken<'_>],
        span: Span,
    ) -> Result<(), GameexeError> {
        let [head] = path else {
            return Err(GameexeError::new(span, "", "expected array index"));
        };
        if head.eq_ignore_ascii_case("CNT") {
            return self.set_count(i32::parse_value(value, head)?, span, head);
        }
        let (first, last) = if let Some((a, b)) = head.split_once('-') {
            (a, b)
        } else {
            (*head, *head)
        };
        let parse_index = |raw: &str| -> Result<usize, GameexeError> {
            if raw.is_empty() || !raw.bytes().all(|b| b.is_ascii_digit()) {
                return Err(GameexeError::new(
                    span.clone(),
                    *head,
                    "expected decimal array index",
                ));
            }
            let index: usize = raw
                .parse()
                .map_err(|_| GameexeError::new(span.clone(), *head, "array index overflow"))?;
            if index >= self.len() {
                return Err(GameexeError::new(
                    span.clone(),
                    *head,
                    "array index outside count",
                ));
            }
            Ok(index)
        };
        let first = parse_index(first)?;
        let last = parse_index(last)?;
        if first > last {
            return Err(GameexeError::new(span, *head, "descending array range"));
        }
        for item in &mut self.as_mut_slice()[first..=last] {
            item.update_value(value, head)?;
        }
        Ok(())
    }
}

pub fn parse<T: GameexeNode>(source: &str) -> Result<T, GameexeError> {
    let mut result = T::gameexe_default();
    let mut line = Vec::new();
    for token in Lexer::new(source) {
        let token = token.map_err(|err| {
            GameexeError::new(err.span, "", format!("lexical error: {:?}", err.kind))
        })?;
        if token.token == Token::Newline {
            apply_line(&mut result, source, &line)?;
            line.clear();
        } else {
            line.push(token);
        }
    }
    apply_line(&mut result, source, &line)?;
    Ok(result)
}

fn apply_line<T: GameexeNode>(
    target: &mut T,
    source: &str,
    line: &[SpannedToken<'_>],
) -> Result<(), GameexeError> {
    // The original reader splits only on LF. A bare CR remains in that line;
    // a successfully matched entry ignores the text after it.
    let line = if let Some(index) = line
        .iter()
        .position(|token| token.token == Token::CarriageReturn)
    {
        if index == 0 {
            return Err(GameexeError::new(
                line[0].span.clone(),
                "",
                "bare carriage return before entry",
            ));
        }
        &line[..index]
    } else {
        line
    };
    if line.is_empty() {
        return Ok(());
    }
    if line.len() < 4 || line[0].token != Token::Hash {
        return Err(GameexeError::new(
            span_of_tokens(line),
            "",
            "expected #PATH = value",
        ));
    }
    let Some(eq_index) = line.iter().position(|token| token.token == Token::Equals) else {
        return Err(GameexeError::new(
            span_of_tokens(line),
            "",
            "expected = after path",
        ));
    };
    if eq_index < 2 || eq_index + 1 >= line.len() {
        return Err(GameexeError::new(
            span_of_tokens(line),
            "",
            "expected #PATH = value",
        ));
    }
    let key_tokens = &line[1..eq_index];
    if gap_has_space_outside_comment(&source[line[0].span.end..key_tokens[0].span.start]) {
        return Err(GameexeError::new(
            key_tokens[0].span.clone(),
            "",
            "space after #",
        ));
    }
    let mut key = String::new();
    let mut previous = "";
    for token in key_tokens {
        let part = match &token.token {
            Token::Identifier(part) | Token::Integer(part) => part.as_ref(),
            _ => {
                return Err(GameexeError::new(
                    token.span.clone(),
                    "",
                    "expected configuration path",
                ));
            }
        };
        if !key.is_empty() && !previous.ends_with(['.', '-']) && !part.starts_with(['.', '-']) {
            return Err(GameexeError::new(
                token.span.clone(),
                "",
                "space inside path name",
            ));
        }
        key.push_str(part);
        previous = part;
    }
    let path: Vec<_> = key.split('.').collect();
    if path.iter().any(|part| part.is_empty()) {
        return Err(GameexeError::new(
            line[1].span.clone(),
            &key,
            "empty path segment",
        ));
    }
    target
        .apply(&path, &line[eq_index + 1..], span_of_tokens(line))
        .map_err(|mut error| {
            error.path = key;
            error
        })
}

fn gap_has_space_outside_comment(gap: &str) -> bool {
    let bytes = gap.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index..].starts_with(b"/*") {
            if let Some(end) = gap[index + 2..].find("*/") {
                index += end + 4;
                continue;
            }
        }
        if matches!(bytes[index], b' ' | b'\t') {
            return true;
        }
        index += 1;
    }
    false
}
