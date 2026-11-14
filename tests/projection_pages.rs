use ores_otel_web_server::persistence::{ProjectionWindow, ReadOnlyProjection};

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
