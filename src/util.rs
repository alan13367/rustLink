//! Utility functions for general-purpose use across the application.

/// Calculate the number of hours from now until a given datetime.
///
/// # Arguments
///
/// * `dt` - The target datetime to calculate the duration for
///
/// # Returns
///
/// The number of hours from now until the given datetime. Negative values
/// indicate the datetime is in the past.
///
/// # Examples
///
/// ```
/// use rustlink::util::hours_from_now;
/// use chrono::Utc;
///
/// let future = Utc::now() + chrono::Duration::hours(24);
/// let hours = hours_from_now(future);
/// assert!(hours > 20);
/// ```
pub fn hours_from_now(dt: chrono::DateTime<chrono::Utc>) -> i64 {
    let now = chrono::Utc::now();
    let duration = dt.signed_duration_since(now);
    duration.num_hours()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_hours_from_now_future() {
        let now = chrono::Utc::now();
        let future = now + Duration::hours(24);
        assert!(hours_from_now(future) > 20);
    }

    #[test]
    fn test_hours_from_now_past() {
        let now = chrono::Utc::now();
        let past = now - Duration::hours(24);
        assert!(hours_from_now(past) < -20);
    }
}
