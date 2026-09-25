use crate::gameexe::{GameexeError, SpannedToken, Token};
use crate::gameexe::parser::GameexeValue;

pub(crate) enum OneToThree<'a> {
    One(&'a [SpannedToken<'a>]),
    Two(&'a [SpannedToken<'a>], &'a [SpannedToken<'a>]),
    Three(
        &'a [SpannedToken<'a>],
        &'a [SpannedToken<'a>],
        &'a [SpannedToken<'a>],
    ),
}

/// Splits a value at commas that are not inside parentheses.
pub(crate) fn split_comma_separated<'a>(
    value: &'a [SpannedToken<'a>],
    path: &str,
) -> Result<Vec<&'a [SpannedToken<'a>]>, GameexeError> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut depth = 0usize;
    for (index, token) in value.iter().enumerate() {
        match token.token {
            Token::LeftParen => depth += 1,
            Token::RightParen => {
                depth = depth.checked_sub(1).ok_or_else(|| {
                    GameexeError::new(token.span.clone(), path, "unexpected closing parenthesis")
                })?
            }
            Token::Comma if depth == 0 => {
                if index == start {
                    return Err(GameexeError::new(token.span.clone(), path, "empty value"));
                }
                parts.push(&value[start..index]);
                start = index + 1;
            }
            _ => {}
        }
    }
    if depth != 0 {
        return Err(GameexeError::new(
            value.last().map_or(0..0, |token| token.span.clone()),
            path,
            "unclosed parenthesis",
        ));
    }
    if start == value.len() {
        return Err(GameexeError::new(
            value.last().map_or(0..0, |token| token.span.clone()),
            path,
            "empty value",
        ));
    }
    parts.push(&value[start..]);
    Ok(parts)
}

pub(crate) fn one_to_three<'a>(
    value: &'a [SpannedToken<'a>],
    path: &str,
) -> Result<OneToThree<'a>, GameexeError> {
    let parts = split_comma_separated(value, path)?;
    match parts.as_slice() {
        [one] => Ok(OneToThree::One(one)),
        [one, two] => Ok(OneToThree::Two(one, two)),
        [one, two, three] => Ok(OneToThree::Three(one, two, three)),
        _ => Err(GameexeError::new(
            value.first().map_or(0..0, |token| token.span.clone()),
            path,
            "expected one to three values",
        )),
    }
}

pub (crate) fn checked_count(
    value: &[SpannedToken<'_>],
    min: i32,
    max: i32,
    path: &str,
) -> Result<usize, GameexeError> {
    let count = i32::parse_value(value, path)?;
    if count < min || count > max {
        return Err(GameexeError::new(
            value.first().map_or(0..0, |token| token.span.clone()),
            path,
            "count outside bounds",
        ));
    }
    Ok(count as usize)
}