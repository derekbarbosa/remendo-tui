//! Generic pagination wrapper for API list responses.

use serde::Deserialize;

/// A paginated list response from the Sashiko API.
///
/// Wraps any item type with pagination metadata. Used by both
/// `GET /api/patchsets` and `GET /api/messages`.
///
/// # Examples
///
/// ```
/// use remendo_tui::models::{Paginated, Patchset};
///
/// // An empty paginated response
/// let empty: Paginated<Patchset> = Paginated {
///     items: vec![],
///     total: 0,
///     page: 1,
///     per_page: 50,
/// };
/// assert!(empty.items.is_empty());
/// assert_eq!(empty.total, 0);
/// ```
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Paginated<T> {
    /// The items on this page.
    pub items: Vec<T>,
    /// Total number of items across all pages.
    #[serde(default)]
    pub total: u32,
    /// Current page number (1-indexed).
    #[serde(default)]
    pub page: u32,
    /// Items per page.
    #[serde(default)]
    pub per_page: u32,
}

impl<T> Paginated<T> {
    /// Total number of pages derived from `total` and `per_page`.
    #[must_use]
    pub fn total_pages(&self) -> u32 {
        if self.per_page == 0 {
            return 1;
        }
        self.total.div_ceil(self.per_page)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::models::Patchset;

    #[test]
    fn deserialize_paginated_patchsets() {
        let json = r#"{
            "items": [{
                "id": 19555,
                "subject": "[PATCH v2 0/4] iio: light: fix null pointer",
                "status": "Pending",
                "author": "dev@example.com",
                "date": 1778690980,
                "total_parts": 4,
                "received_parts": 4,
                "subsystems": ["LKML", "linux-iio"],
                "findings_low": 0,
                "findings_medium": 1,
                "findings_high": 0,
                "findings_critical": 0
            }],
            "total": 19558,
            "page": 1,
            "per_page": 50
        }"#;
        let paginated: Paginated<Patchset> =
            serde_json::from_str(json).expect("deserialize paginated");
        assert_eq!(paginated.items.len(), 1);
        assert_eq!(paginated.total, 19558);
        assert_eq!(paginated.page, 1);
        assert_eq!(paginated.per_page, 50);
    }

    #[test]
    fn deserialize_empty_paginated() {
        let json = r#"{"items": [], "total": 0, "page": 1, "per_page": 50}"#;
        let paginated: Paginated<Patchset> =
            serde_json::from_str(json).expect("deserialize empty paginated");
        assert!(paginated.items.is_empty());
        assert_eq!(paginated.total, 0);
    }
}
