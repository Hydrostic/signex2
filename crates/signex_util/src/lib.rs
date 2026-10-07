use std::path::{Component, Path};

/// Returns whether a path is relative and contains no parent-directory component.
pub fn is_relative(path: &Path) -> bool {
    !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_) | Component::CurDir))
}

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
    use super::{fullwidth, is_relative};
    use std::path::Path;

    #[test]
    fn relative_path_rejects_parent_and_absolute_components() {
        assert!(is_relative(Path::new("assets/./hero.g00")));
        assert!(!is_relative(Path::new("../hero.g00")));
        assert!(!is_relative(Path::new("/assets/hero.g00")));
    }

    #[test]
    fn converts_every_decimal_digit() {
        assert_eq!(fullwidth(0), "０");
        assert_eq!(fullwidth(120), "１２０");
    }
}
