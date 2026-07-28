/// Maximum allowed nesting depth for shapes.
///
/// This bounds recursive structures like `Parameterized(Parameterized(...))`
/// and `Record(Record(...))` to prevent stack overflow or memory exhaustion.
pub const MAX_SHAPE_DEPTH: u32 = 64;

/// A structural shape for a node's domain or codomain.
///
/// Shapes are structural and permit an `Opaque` sentinel for cases where
/// extraction is not possible (dynamic, untyped, no annotations).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Shape {
    /// A scalar value (e.g., `int`, `string`, `bool`).
    Scalar,
    /// A structured record/product type.
    Record(Vec<Shape>),
    /// A tagged union/sum type.
    Union(Vec<Shape>),
    /// A function/callable shape with domain and codomain.
    Function(Box<Shape>, Box<Shape>),
    /// A reference/pointer to a shape.
    Ref(Box<Shape>),
    /// A generic/parameterized shape.
    Parameterized {
        /// The base shape identifier.
        base: String,
        /// Type parameters.
        parameters: Vec<Shape>,
    },
    /// Sentinel for shapes that cannot be extracted (e.g., fully dynamic,
    /// untyped, no annotations available).
    Opaque,
    /// The bottom/never type (e.g., Rust's `!`). A diverging computation
    /// that does not produce a value. Distinct from `Scalar` so callers can
    /// flag unreachable continuations.
    Bottom,
}

impl Shape {
    /// Compute the maximum nesting depth of this shape.
    ///
    /// Leaf variants (Scalar, Opaque) have depth 0.
    /// Compound variants have depth = 1 + max(depth of children).
    pub fn depth(&self) -> u32 {
        match self {
            Shape::Scalar | Shape::Opaque | Shape::Bottom => 0,
            Shape::Record(fields) | Shape::Union(fields) => {
                1 + fields.iter().map(Shape::depth).max().unwrap_or(0)
            }
            Shape::Function(dom, cod) => 1 + dom.depth().max(cod.depth()),
            Shape::Ref(inner) => 1 + inner.depth(),
            Shape::Parameterized { parameters, .. } => {
                1 + parameters.iter().map(Shape::depth).max().unwrap_or(0)
            }
        }
    }

    /// Returns `true` if the shape depth is within the allowed limit.
    pub fn is_depth_valid(&self) -> bool {
        self.depth() <= MAX_SHAPE_DEPTH
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shape_scalar() {
        assert_eq!(
            serde_json::to_value(Shape::Scalar).unwrap(),
            serde_json::json!("scalar")
        );
    }

    #[test]
    fn shape_opaque() {
        assert_eq!(
            serde_json::to_value(Shape::Opaque).unwrap(),
            serde_json::json!("opaque")
        );
    }

    #[test]
    fn shape_bottom() {
        assert_eq!(
            serde_json::to_value(Shape::Bottom).unwrap(),
            serde_json::json!("bottom")
        );
    }

    #[test]
    fn shape_depth_bottom() {
        assert_eq!(Shape::Bottom.depth(), 0);
    }

    #[test]
    fn shape_record() {
        let s = Shape::Record(vec![Shape::Scalar, Shape::Scalar]);
        assert_eq!(
            serde_json::to_value(s).unwrap(),
            serde_json::json!({"record": ["scalar", "scalar"]})
        );
    }

    #[test]
    fn shape_function() {
        let s = Shape::Function(Box::new(Shape::Scalar), Box::new(Shape::Opaque));
        let json = serde_json::to_value(s).unwrap();
        assert_eq!(json["function"][0], "scalar");
        assert_eq!(json["function"][1], "opaque");
    }

    #[test]
    fn shape_parameterized() {
        let s = Shape::Parameterized {
            base: "Vec".into(),
            parameters: vec![Shape::Scalar],
        };
        let json = serde_json::to_value(s).unwrap();
        assert_eq!(json["parameterized"]["base"], "Vec");
        assert_eq!(
            json["parameterized"]["parameters"],
            serde_json::json!(["scalar"])
        );
    }

    #[test]
    fn shape_depth_scalar() {
        assert_eq!(Shape::Scalar.depth(), 0);
    }

    #[test]
    fn shape_depth_opaque() {
        assert_eq!(Shape::Opaque.depth(), 0);
    }

    #[test]
    fn shape_depth_record() {
        let s = Shape::Record(vec![Shape::Scalar, Shape::Scalar]);
        assert_eq!(s.depth(), 1);
    }

    #[test]
    fn shape_depth_nested_record() {
        let inner = Shape::Record(vec![Shape::Scalar]);
        let outer = Shape::Record(vec![inner]);
        assert_eq!(outer.depth(), 2);
    }

    #[test]
    fn shape_depth_parameterized_nested() {
        let inner = Shape::Parameterized {
            base: "Option".into(),
            parameters: vec![Shape::Scalar],
        };
        let outer = Shape::Parameterized {
            base: "Vec".into(),
            parameters: vec![inner],
        };
        assert_eq!(outer.depth(), 2);
    }

    #[test]
    fn shape_depth_within_limit() {
        let mut s = Shape::Scalar;
        for _ in 0..10 {
            s = Shape::Record(vec![s]);
        }
        assert!(s.is_depth_valid());
    }

    #[test]
    fn shape_depth_beyond_limit() {
        let mut s = Shape::Scalar;
        for _ in 0..(MAX_SHAPE_DEPTH + 1) {
            s = Shape::Record(vec![s]);
        }
        assert!(!s.is_depth_valid());
    }
}
