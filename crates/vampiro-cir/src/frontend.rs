use std::path::Path;

use crate::cir::CirGraph;
use crate::error::CirError;

/// The extraction contract that every frontend must implement.
///
/// A frontend translates source code in a specific language into a `CirGraph`.
/// The platform loads frontends through this trait, verifying conformance
/// before any graph is used for analysis.
pub trait Frontend {
    /// The language identifier for this frontend (e.g., `"rust"`, `"python"`).
    fn language(&self) -> &'static str;

    /// Extract a CIR graph from source code.
    ///
    /// # Errors
    ///
    /// Returns `CirError::Extraction` if the source cannot be parsed or the
    /// graph cannot be constructed. Returns `CirError::EffectDepthExceeded`
    /// or `CirError::ShapeDepthExceeded` if the extracted graph exceeds
    /// resource limits.
    fn extract(&self, source: &str, path: &Path) -> Result<CirGraph, CirError>;
}

/// A no-op frontend that always returns an empty graph.
///
/// Useful as a placeholder during development or for testing.
pub struct NullFrontend;

impl Frontend for NullFrontend {
    fn language(&self) -> &'static str {
        "null"
    }

    fn extract(&self, _source: &str, _path: &Path) -> Result<CirGraph, CirError> {
        Ok(CirGraph::new(""))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    struct TestFrontend;

    impl Frontend for TestFrontend {
        fn language(&self) -> &'static str {
            "test"
        }

        fn extract(&self, _source: &str, _path: &Path) -> Result<CirGraph, CirError> {
            let graph = CirGraph::new(_path.to_string_lossy());
            Ok(graph)
        }
    }

    #[test]
    fn test_frontend_implements_trait() {
        let frontend = TestFrontend;
        assert_eq!(frontend.language(), "test");
    }

    #[test]
    fn null_frontend_returns_empty_graph() {
        let frontend = NullFrontend;
        let graph = frontend
            .extract("fn main() {}", Path::new("test.rs"))
            .unwrap();
        assert_eq!(graph.nodes.len(), 0);
        assert_eq!(graph.edges.len(), 0);
    }

    #[test]
    fn frontend_language_is_static() {
        let frontend = TestFrontend;
        // The language string must be 'static — this compiles only if it is.
        let _lang: &'static str = frontend.language();
    }
}
