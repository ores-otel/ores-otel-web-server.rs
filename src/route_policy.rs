#![forbid(unsafe_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteClass {
    Page,
    Static,
    Docs,
}

pub const STATIC_PREFIX: &str = "/static";
pub const DOCS_PREFIX: &str = "/_/docs";

pub fn classify_request_path(path: &str) -> RouteClass {
    if path == STATIC_PREFIX || path.starts_with("/static/") {
        return RouteClass::Static;
    }
    if path == DOCS_PREFIX || path.starts_with("/_/docs/") {
        return RouteClass::Docs;
    }
    RouteClass::Page
}

pub fn may_probe_filesystem(class: RouteClass) -> bool {
    matches!(class, RouteClass::Static)
}

#[cfg(test)]
mod tests {
    use super::{classify_request_path, may_probe_filesystem, RouteClass};

    #[test]
    fn static_namespace_is_classified_before_page_routing() {
        assert_eq!(classify_request_path("/static"), RouteClass::Static);
        assert_eq!(classify_request_path("/static/app.js"), RouteClass::Static);
        assert!(may_probe_filesystem(RouteClass::Static));
    }

    #[test]
    fn docs_namespace_never_falls_into_static_lookup() {
        assert_eq!(classify_request_path("/_/docs"), RouteClass::Docs);
        assert_eq!(classify_request_path("/_/docs/openapi.json"), RouteClass::Docs);
        assert!(!may_probe_filesystem(RouteClass::Docs));
    }

    #[test]
    fn ordinary_pages_never_probe_the_filesystem() {
        for path in ["/", "/health", "/users/alex", "/staticish", "/_/documentation"] {
            assert_eq!(classify_request_path(path), RouteClass::Page, "{path}");
            assert!(!may_probe_filesystem(RouteClass::Page));
        }
    }
}
