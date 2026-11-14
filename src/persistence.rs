#![forbid(unsafe_code)]

//! Read-only projection pagination. The ORM remains responsible for
//! authentication and database access; this layer only computes safe bounds.

#[derive(Clone, Debug, Default)]
pub struct ReadOnlyProjection {
    pub rows: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProjectionWindow {
    pub offset: usize,
    pub count: usize,
    pub next_offset: Option<usize>,
}

impl ReadOnlyProjection {
    /// Computes a bounded page without performing a query or allocating.
    /// Returns None for zero or unbounded page sizes.
    #[must_use]
    pub fn page_window(&self, offset: usize, page_size: usize) -> Option<ProjectionWindow> {
        const MAX_PAGE_SIZE: usize = 1_000;
        if page_size == 0 || page_size > MAX_PAGE_SIZE {
            return None;
        }
        let start = offset.min(self.rows);
        let end = start.saturating_add(page_size).min(self.rows);
        Some(ProjectionWindow {
            offset: start,
            count: end - start,
            next_offset: (end < self.rows).then_some(end),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn walks_bounded_pages_without_overfetching() {
        let projection = ReadOnlyProjection { rows: 5 };
        assert_eq!(
            projection.page_window(0, 2),
            Some(ProjectionWindow {
                offset: 0,
                count: 2,
                next_offset: Some(2),
            })
        );
        assert_eq!(
            projection.page_window(4, 2),
            Some(ProjectionWindow {
                offset: 4,
                count: 1,
                next_offset: None,
            })
        );
    }

    #[test]
    fn rejects_unbounded_and_zero_page_sizes() {
        let projection = ReadOnlyProjection { rows: 9_999 };
        assert_eq!(projection.page_window(0, 0), None);
        assert_eq!(projection.page_window(0, 1_001), None);
        assert!(projection.page_window(0, 1_000).is_some());
    }

    #[test]
    fn clamps_offsets_and_does_not_overflow_extreme_row_counts() {
        let projection = ReadOnlyProjection { rows: usize::MAX };
        let result = projection.page_window(usize::MAX, 2).unwrap();
        assert_eq!(result.count, 0);
        assert_eq!(result.next_offset, None);
        let empty = ReadOnlyProjection { rows: 0 };
        assert_eq!(empty.page_window(500, 10).unwrap().count, 0);
    }
}
