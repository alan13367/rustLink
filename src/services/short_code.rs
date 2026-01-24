use crate::db::Repository;
use crate::error::{AppError, AppResult};

/// Character set for generating short codes.
const ALPHABET_CHARS: &[char] = &[
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9',
    'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M',
    'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z',
    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm',
    'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
];

/// Service for generating unique short codes.
pub struct ShortCodeService;

impl ShortCodeService {
    /// Generate a unique short code that doesn't already exist in the database.
    ///
    /// # Arguments
    ///
    /// * `length` - The desired length of the short code
    /// * `max_attempts` - Maximum number of attempts to generate a unique code
    /// * `repository` - Database repository to check for existing codes
    ///
    /// # Returns
    ///
    /// A unique short code string, or an error if unable to generate a unique code
    /// after max_attempts.
    ///
    /// # Errors
    ///
    /// Returns `AppError::ShortCodeGenerationFailed` if unable to generate a unique
    /// code within the specified number of attempts.
    pub async fn generate_short_code(
        length: usize,
        max_attempts: u32,
        repository: &Repository,
    ) -> AppResult<String> {
        for _ in 0..max_attempts {
            let code = nanoid::nanoid!(length, ALPHABET_CHARS);

            if !repository.short_code_exists(&code).await? {
                return Ok(code);
            }
        }

        Err(AppError::ShortCodeGenerationFailed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alphabet_chars_const() {
        // Verify the alphabet has 62 characters (0-9, A-Z, a-z)
        assert_eq!(ALPHABET_CHARS.len(), 62);
    }

    #[test]
    fn test_alphabet_chars_unique() {
        // Verify all characters are unique
        let unique: std::collections::HashSet<_> = ALPHABET_CHARS.iter().collect();
        assert_eq!(unique.len(), ALPHABET_CHARS.len());
    }
}
