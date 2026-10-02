//! Library core for `decider-adr`.
//!
//! Keep logic here so it can be unit tested and documented. The binary in
//! `main.rs` stays a thin wrapper that handles process concerns only.

#![warn(missing_docs)]

/// Errors returned by this crate.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Error {
    /// The supplied name was empty or only whitespace.
    #[error("name must not be empty")]
    EmptyName,
}

/// Build a greeting for `name`.
///
/// # Errors
///
/// Returns [`Error::EmptyName`] if `name` is empty or only whitespace.
///
/// # Examples
///
/// ```
/// let greeting = decider_adr::greet("Ada")?;
/// assert_eq!(greeting, "Hello, Ada!");
/// # Ok::<(), decider_adr::Error>(())
/// ```
pub fn greet(name: &str) -> Result<String, Error> {
    let name = name.trim();
    if name.is_empty() {
        return Err(Error::EmptyName);
    }
    Ok(format!("Hello, {name}!"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn greets_by_name() {
        assert_eq!(greet("Ada").unwrap(), "Hello, Ada!");
    }

    #[test]
    fn trims_whitespace() {
        assert_eq!(greet("  Ada ").unwrap(), "Hello, Ada!");
    }

    #[test]
    fn rejects_blank_name() {
        assert_eq!(greet("   "), Err(Error::EmptyName));
    }
}
