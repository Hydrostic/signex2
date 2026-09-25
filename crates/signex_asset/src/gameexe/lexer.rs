//! Lexer for the text form of Siglus Gameexe.ini configuration files.

use super::types::{LexError, LexErrorKind, SpannedToken, Token};
use std::borrow::Cow;

/// Scans a UTF-8 Gameexe.ini file from left to right.
///
/// Spaces and tabs are ignored, while line endings are returned as tokens so
/// a later parser can determine where each configuration entry ends. The
/// iterator stops after its first error.
#[derive(Debug, Clone)]
pub struct Lexer<'a> {
    source: &'a str,
    offset: usize,
    finished: bool,
    in_block_comment: bool,
    after_equals: bool,
}

impl<'a> Lexer<'a> {
    /// Creates a lexer borrowing `source` for the lifetime of its tokens.
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            offset: 0,
            finished: false,
            in_block_comment: false,
            after_equals: false,
        }
    }

    /// Returns the byte offset where the next scan will begin.
    pub fn offset(&self) -> usize {
        self.offset
    }

    fn token(&self, token: Token<'a>, start: usize) -> SpannedToken<'a> {
        SpannedToken {
            token,
            span: start..self.offset,
        }
    }

    fn scan_string(&mut self, start: usize) -> Result<SpannedToken<'a>, LexError> {
        // The opening quote has already been consumed. An escaped quote does
        // not close the string; all escape text is deliberately left intact.
        while self.offset < self.source.len() {
            let ch = self.source[self.offset..].chars().next().unwrap();
            match ch {
                '"' => {
                    let content = &self.source[start + 1..self.offset];
                    self.offset += 1;
                    return Ok(self.token(Token::String(content), start));
                }
                '\\' => {
                    self.offset += 1;
                    if let Some(next) = self.source[self.offset..].chars().next() {
                        if next != '\n' && next != '\r' {
                            self.offset += next.len_utf8();
                        }
                    }
                }
                '\n' => break,
                _ => self.offset += ch.len_utf8(),
            }
        }
        Err(LexError {
            kind: LexErrorKind::UnterminatedString,
            span: start..self.offset,
        })
    }

    fn scan_word(&mut self, start: usize) -> SpannedToken<'a> {
        // Keys can contain numeric segments such as OBJECT.000-099.USE.
        let mut joined = None::<String>;
        let mut segment_start = start;
        while let Some(ch) = self.source[self.offset..].chars().next() {
            if self.source[self.offset..].starts_with("/*") {
                joined
                    .get_or_insert_with(String::new)
                    .push_str(&self.source[segment_start..self.offset]);
                self.offset += 2;
                self.in_block_comment = true;
                while self.offset < self.source.len() {
                    if self.source[self.offset..].starts_with("*/") {
                        self.offset += 2;
                        self.in_block_comment = false;
                        break;
                    }
                    if self.source.as_bytes()[self.offset] == b'\n' {
                        break;
                    }
                    self.offset += self.source[self.offset..]
                        .chars()
                        .next()
                        .unwrap()
                        .len_utf8();
                }
                segment_start = self.offset;
                if self.in_block_comment {
                    break;
                }
                continue;
            }
            if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | '-') {
                self.offset += ch.len_utf8();
            } else {
                break;
            }
        }
        let word = if let Some(mut joined) = joined {
            joined.push_str(&self.source[segment_start..self.offset]);
            Cow::Owned(joined)
        } else {
            Cow::Borrowed(&self.source[start..self.offset])
        };
        self.token(Token::Identifier(word), start)
    }

    fn scan_signed_integer(&mut self, start: usize) -> Result<SpannedToken<'a>, LexError> {
        let mut normalized = String::from(&self.source[start..start + 1]);
        let mut has_digit = self.source.as_bytes()[start].is_ascii_digit();
        while self.offset < self.source.len() {
            if !has_digit && matches!(self.source.as_bytes()[self.offset], b' ' | b'\t') {
                self.offset += 1;
                continue;
            }
            if self.source[self.offset..].starts_with("/*") {
                self.offset += 2;
                self.in_block_comment = true;
                while self.offset < self.source.len() {
                    if self.source[self.offset..].starts_with("*/") {
                        self.offset += 2;
                        self.in_block_comment = false;
                        break;
                    }
                    if self.source.as_bytes()[self.offset] == b'\n' {
                        break;
                    }
                    self.offset += self.source[self.offset..]
                        .chars()
                        .next()
                        .unwrap()
                        .len_utf8();
                }
                if self.in_block_comment {
                    break;
                }
                continue;
            }
            if self.source.as_bytes()[self.offset].is_ascii_digit() {
                normalized.push(self.source.as_bytes()[self.offset] as char);
                self.offset += 1;
                has_digit = true;
            } else {
                break;
            }
        }
        if !has_digit {
            return Err(LexError {
                kind: LexErrorKind::UnexpectedCharacter,
                span: start..start + 1,
            });
        }
        let word = if self.source[start..self.offset] == normalized {
            Cow::Borrowed(&self.source[start..self.offset])
        } else {
            Cow::Owned(normalized)
        };
        Ok(self.token(Token::Integer(word), start))
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Result<SpannedToken<'a>, LexError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        // Comments may span lines; keep line endings as entry boundaries.
        loop {
            if self.in_block_comment {
                if self.offset == self.source.len() {
                    self.finished = true;
                    return Some(Err(LexError {
                        kind: LexErrorKind::UnterminatedComment,
                        span: self.offset..self.offset,
                    }));
                }
                if self.source[self.offset..].starts_with("*/") {
                    self.offset += 2;
                    self.in_block_comment = false;
                    continue;
                }
                if self.source.as_bytes()[self.offset] == b'\n' {
                    break;
                }
                self.offset += self.source[self.offset..]
                    .chars()
                    .next()
                    .unwrap()
                    .len_utf8();
                continue;
            }
            if self.offset == self.source.len() {
                break;
            }
            if self.source[self.offset..].starts_with("/*") {
                self.offset += 2;
                self.in_block_comment = true;
                continue;
            }
            if self.source[self.offset..].starts_with("//")
                || self.source[self.offset..].starts_with(';')
            {
                while self.offset < self.source.len()
                    && self.source.as_bytes()[self.offset] != b'\n'
                {
                    self.offset += self.source[self.offset..]
                        .chars()
                        .next()
                        .unwrap()
                        .len_utf8();
                }
                continue;
            }
            let ch = self.source[self.offset..].chars().next().unwrap();
            if ch == ' ' || ch == '\t' {
                self.offset += 1;
            } else {
                break;
            }
        }
        if self.offset == self.source.len() {
            self.finished = true;
            return None;
        }

        let start = self.offset;
        let ch = self.source[start..].chars().next().unwrap();
        self.offset += ch.len_utf8();
        let result = match ch {
            '#' => Ok(self.token(Token::Hash, start)),
            '=' => {
                self.after_equals = true;
                Ok(self.token(Token::Equals, start))
            }
            ',' => Ok(self.token(Token::Comma, start)),
            '(' => Ok(self.token(Token::LeftParen, start)),
            ')' => Ok(self.token(Token::RightParen, start)),
            '\n' => {
                self.after_equals = false;
                Ok(self.token(Token::Newline, start))
            }
            '\r' => {
                if self.source[self.offset..].starts_with('\n') {
                    self.offset += 1;
                    self.after_equals = false;
                    Ok(self.token(Token::Newline, start))
                } else {
                    Ok(self.token(Token::CarriageReturn, start))
                }
            }
            '"' => self.scan_string(start),
            c if self.after_equals && (c.is_ascii_digit() || matches!(c, '+' | '-')) => {
                self.scan_signed_integer(start)
            }
            c if c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-') => {
                Ok(self.scan_word(start))
            }
            _ => Err(LexError {
                kind: LexErrorKind::UnexpectedCharacter,
                span: start..self.offset,
            }),
        };
        if result.is_err() {
            self.finished = true;
        }
        Some(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(input: &str) -> Vec<Token<'_>> {
        Lexer::new(input)
            .map(|result| result.unwrap().token)
            .collect()
    }

    #[test]
    fn scans_values_and_preserves_spelling() {
        assert_eq!(
            tokens("#CONFIG.FILTER_COLOR = 070, -8, \"日本語\\\\n\"\r\n"),
            vec![
                Token::Hash,
                Token::Identifier("CONFIG.FILTER_COLOR".into()),
                Token::Equals,
                Token::Integer("070".into()),
                Token::Comma,
                Token::Integer("-8".into()),
                Token::Comma,
                Token::String("日本語\\\\n"),
                Token::Newline,
            ]
        );
    }

    #[test]
    fn scans_range_keys_and_grouped_values() {
        assert_eq!(
            tokens("#OBJECT.000-099.USE = 1\n#SHAKE.000 = (0,-8,64)(2,2,32)"),
            vec![
                Token::Hash,
                Token::Identifier("OBJECT.000-099.USE".into()),
                Token::Equals,
                Token::Integer("1".into()),
                Token::Newline,
                Token::Hash,
                Token::Identifier("SHAKE.000".into()),
                Token::Equals,
                Token::LeftParen,
                Token::Integer("0".into()),
                Token::Comma,
                Token::Integer("-8".into()),
                Token::Comma,
                Token::Integer("64".into()),
                Token::RightParen,
                Token::LeftParen,
                Token::Integer("2".into()),
                Token::Comma,
                Token::Integer("2".into()),
                Token::Comma,
                Token::Integer("32".into()),
                Token::RightParen,
            ]
        );
    }

    #[test]
    fn spans_include_quotes_and_full_line_endings() {
        let source = "#X=\"界\"\r\n";
        let found: Vec<_> = Lexer::new(source).map(Result::unwrap).collect();
        assert_eq!(found[3].span, 3..8);
        assert_eq!(&source[found[3].span.clone()], "\"界\"");
        assert_eq!(found[4].span, 8..10);
    }

    #[test]
    fn escaped_quote_does_not_end_string() {
        assert_eq!(
            tokens("#X=\"a\\\"b\""),
            vec![
                Token::Hash,
                Token::Identifier("X".into()),
                Token::Equals,
                Token::String("a\\\"b"),
            ]
        );
    }

    #[test]
    fn reports_errors_once() {
        let mut lexer = Lexer::new("#X=\"unfinished\n#Y=1");
        for _ in 0..3 {
            lexer.next().unwrap().unwrap();
        }
        assert_eq!(
            lexer.next().unwrap().unwrap_err().kind,
            LexErrorKind::UnterminatedString
        );
        assert!(lexer.next().is_none());

        let mut lexer = Lexer::new("#X=@");
        for _ in 0..3 {
            lexer.next().unwrap().unwrap();
        }
        assert_eq!(lexer.next().unwrap().unwrap_err().span, 3..4);
        assert!(lexer.next().is_none());
    }

    #[test]
    fn scans_reference_file_when_available() {
        // Set this variable to an existing Gameexe.ini path to check a real
        // game file without copying its contents into the repository.
        let Ok(path) = std::env::var("SIGNEX_GAMEEXE_FIXTURE") else {
            return;
        };
        let source = std::fs::read_to_string(path).unwrap();
        let mut entries = 0;
        for result in Lexer::new(&source) {
            if result.unwrap().token == Token::Hash {
                entries += 1;
            }
        }
        assert!(entries > 0);
    }

    #[test]
    fn accepts_signed_values_and_splices_block_comments() {
        let source = "#SEL/* note */BTN.CNT=+ 1/*x*/5,- 1";
        assert_eq!(
            tokens(source),
            vec![
                Token::Hash,
                Token::Identifier("SELBTN.CNT".into()),
                Token::Equals,
                Token::Integer("+15".into()),
                Token::Comma,
                Token::Integer("-1".into()),
            ]
        );
        let found: Vec<_> = Lexer::new(source).map(Result::unwrap).collect();
        assert_eq!(&source[found[1].span.clone()], "SEL/* note */BTN.CNT");
        assert_eq!(&source[found[3].span.clone()], "+ 1/*x*/5");
    }

    #[test]
    fn numeric_path_segments_are_words_but_values_are_integers() {
        assert_eq!(
            tokens("#OBJECT . 001 = 0/*x*/70"),
            vec![
                Token::Hash,
                Token::Identifier("OBJECT".into()),
                Token::Identifier(".".into()),
                Token::Identifier("001".into()),
                Token::Equals,
                Token::Integer("070".into()),
            ]
        );
    }

    #[test]
    fn bare_carriage_return_is_not_a_line_break() {
        assert_eq!(
            tokens("#X=1\r#Y=2\r\n"),
            vec![
                Token::Hash,
                Token::Identifier("X".into()),
                Token::Equals,
                Token::Integer("1".into()),
                Token::CarriageReturn,
                Token::Hash,
                Token::Identifier("Y".into()),
                Token::Equals,
                Token::Integer("2".into()),
                Token::Newline,
            ]
        );
    }
}
