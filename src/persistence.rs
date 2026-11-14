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
