/// Converts decimal digits of an index to their fullwidth forms.
pub fn fullwidth(index: usize) -> String {
    index
        .to_string()
        .chars()
        .map(|digit| char::from_u32(digit as u32 - '0' as u32 + '０' as u32).unwrap())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::fullwidth;

    #[test]
    fn converts_every_decimal_digit() {
        assert_eq!(fullwidth(0), "０");
        assert_eq!(fullwidth(120), "１２０");
    }
}
