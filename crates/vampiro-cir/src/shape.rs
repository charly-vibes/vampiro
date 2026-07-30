/// Maximum allowed nesting depth for shapes.
///
/// This bounds recursive structures like `Parameterized(Parameterized(...))`
/// and `Record(Record(...))` to prevent stack overflow or memory exhaustion.
pub const MAX_SHAPE_DEPTH: u32 = 64;

/// A fine-grained scalar kind for type-aware shape comparison.
///
/// Replaces the coarse `Shape::Scalar(ScalarKind::Unit)` unit variant. Unknown scalar types
/// (fallback in frontends) use `Unit` as the default.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ScalarKind {
    /// An integer type (i8, u32, usize, etc.).
    #[serde(rename = "int")]
    Int,
    /// A floating-point type (f32, f64).
    #[serde(rename = "float")]
    Float,
    /// A boolean type.
    #[serde(rename = "bool")]
    Bool,
    /// A character type.
    #[serde(rename = "char")]
    Char,
    /// A string type (String, &str).
    #[serde(rename = "string")]
    String,
    /// The unit/void type, or a generic/unknown scalar.
    #[serde(rename = "unit")]
    Unit,
}

/// A structural shape for a node's domain or codomain.
///
/// Shapes are structural and permit an `Opaque` sentinel for cases where
/// extraction is not dynamic (dynamic).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Shape {
    /// A scalar value of a specific kind (int, float, bool, char, string, unit).
    Scalar(ScalarKind),
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
            Shape::Scalar(_) | Shape::Opaque | Shape::Bottom => 0,
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

    /// Canonicalize this shape for structural comparison, deduplication,
    /// and fixture serialization.
    ///
    /// See `docs/decisions/shape-canonicalization.md` for the full decision.
    /// In short:
    /// - `Union` arms are sorted by their canonical serialization (unordered-set
    ///   semantics; positional sums are modeled as `Parameterized`, whose order
    ///   is preserved).
    /// - `Record` fields are sorted by canonical serialization (records are
    ///   structural products with no field names at this representation level).
    /// - `Opaque` and `Bottom` are preserved as leaves.
    /// - `Parameterized` parameters keep their positional order; each is
    ///   normalized recursively.
    /// - `Function` and `Ref` normalize their inner shapes.
    ///
    /// Normalization is idempotent: `normalize(normalize(s)) == normalize(s)`.
    pub fn normalize(&self) -> Shape {
        match self {
            Shape::Scalar(_) | Shape::Opaque | Shape::Bottom => self.clone(),
            Shape::Record(fields) => {
                let normalized: Vec<Shape> = fields.iter().map(Shape::normalize).collect();
                let mut keyed: Vec<(String, Shape)> = normalized
                    .into_iter()
                    .map(|s| (canonical_json(&s), s))
                    .collect();
                keyed.sort_by(|a, b| a.0.cmp(&b.0));
                Shape::Record(keyed.into_iter().map(|(_, s)| s).collect())
            }
            Shape::Union(arms) => {
                let normalized: Vec<Shape> = arms.iter().map(Shape::normalize).collect();
                let mut keyed: Vec<(String, Shape)> = normalized
                    .into_iter()
                    .map(|s| (canonical_json(&s), s))
                    .collect();
                keyed.sort_by(|a, b| a.0.cmp(&b.0));
                Shape::Union(keyed.into_iter().map(|(_, s)| s).collect())
            }
            Shape::Function(dom, cod) => {
                Shape::Function(Box::new(dom.normalize()), Box::new(cod.normalize()))
            }
            Shape::Ref(inner) => Shape::Ref(Box::new(inner.normalize())),
            Shape::Parameterized { base, parameters } => Shape::Parameterized {
                base: base.clone(),
                parameters: parameters.iter().map(Shape::normalize).collect(),
            },
        }
    }

    /// Canonical compact-JSON serialization of the normalized shape.
    ///
    /// This is the form used by [`canonical_hash`](Self::canonical_hash) and by
    /// any cross-version fixture comparison (REQ-29). It is deterministic and
    /// content-sensitive.
    pub fn to_canonical_json(&self) -> String {
        canonical_json(&self.normalize())
    }

    /// A 128-bit (32 hex char) content hash of the normalized shape, using the
    /// same scheme as `StableId` (SHA-256 truncated to 128 bits, hex-encoded).
    ///
    /// This is the shape-hash component of the REQ-24 dedupe identity. Hash
    /// equality is a candidate match only; callers MUST confirm with a
    /// structural comparison before suppressing a finding or accepting a
    /// fixture (see decision §3 Collision handling).
    pub fn canonical_hash(&self) -> String {
        use sha2::Digest;
        let json = self.to_canonical_json();
        let digest = sha2::Sha256::digest(json.as_bytes());
        // Truncate to 128 bits (16 bytes) and hex-encode.
        to_hex_lower(&digest[..16])
    }

    /// Extract the expected shape at a parameter slot position.
    ///
    /// Returns `None` when the shape is not slot-indexable (Scalar, Opaque,
    /// Bottom, Union), when the slot index is out of bounds, or when the
    /// shape provides no parameter structure.
    ///
    /// - `Record(fields)`: index into the fields by position.
    /// - `Function(dom, _)`: the domain is itself the parameter shape;
    ///   returns the domain for slot 0, or `None` for higher slots.
    /// - `Parameterized { parameters, .. }`: index into parameters.
    pub fn domain_slot(&self, slot: u32) -> Option<&Shape> {
        match self {
            Shape::Record(fields) => fields.get(slot as usize),
            Shape::Function(dom, _) if slot == 0 => Some(dom.as_ref()),
            Shape::Parameterized { parameters, .. } => parameters.get(slot as usize),
            _ => None,
        }
    }
}

impl ScalarKind {
    /// All `ScalarKind` variants for exhaustive iteration.
    pub const ALL: &'static [ScalarKind] = &[
        ScalarKind::Int,
        ScalarKind::Float,
        ScalarKind::Bool,
        ScalarKind::Char,
        ScalarKind::String,
        ScalarKind::Unit,
    ];
}

/// Lowercase hex encoding of a byte slice (no external `hex` dependency).
fn to_hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

/// Deterministic compact-JSON serialization of a (assumed-normalized) shape.
///
/// Compact form: no whitespace, field order as declared by the `Shape` serde
/// derive (no object needs key sorting at this representation level).
fn canonical_json(shape: &Shape) -> String {
    serde_json::to_string(shape).expect("Shape serialization is infallible")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shape_scalar() {
        assert_eq!(
            serde_json::to_value(Shape::Scalar(ScalarKind::Unit)).unwrap(),
            serde_json::json!({ "scalar": "unit" })
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
        let s = Shape::Record(vec![
            Shape::Scalar(ScalarKind::Unit),
            Shape::Scalar(ScalarKind::Unit),
        ]);
        assert_eq!(
            serde_json::to_value(s).unwrap(),
            serde_json::json!({ "record": [{ "scalar": "unit" }, { "scalar": "unit" }] })
        );
    }

    #[test]
    fn shape_function() {
        let s = Shape::Function(
            Box::new(Shape::Scalar(ScalarKind::Unit)),
            Box::new(Shape::Opaque),
        );
        let json = serde_json::to_value(s).unwrap();
        assert_eq!(json["function"][0], serde_json::json!({"scalar": "unit"}));
        assert_eq!(json["function"][1], "opaque");
    }

    #[test]
    fn shape_parameterized() {
        let s = Shape::Parameterized {
            base: "Vec".into(),
            parameters: vec![Shape::Scalar(ScalarKind::Unit)],
        };
        let json = serde_json::to_value(s).unwrap();
        assert_eq!(json["parameterized"]["base"], "Vec");
        assert_eq!(
            json["parameterized"]["parameters"],
            serde_json::json!([{ "scalar": "unit" }])
        );
    }

    #[test]
    fn shape_depth_scalar_leaf() {
        assert_eq!(Shape::Scalar(ScalarKind::Unit).depth(), 0);
    }

    #[test]
    fn shape_depth_opaque() {
        assert_eq!(Shape::Opaque.depth(), 0);
    }

    #[test]
    fn shape_depth_record() {
        let s = Shape::Record(vec![
            Shape::Scalar(ScalarKind::Unit),
            Shape::Scalar(ScalarKind::Unit),
        ]);
        assert_eq!(s.depth(), 1);
    }

    #[test]
    fn shape_depth_nested_record() {
        let inner = Shape::Record(vec![Shape::Scalar(ScalarKind::Unit)]);
        let outer = Shape::Record(vec![inner]);
        assert_eq!(outer.depth(), 2);
    }

    #[test]
    fn shape_depth_parameterized_nested() {
        let inner = Shape::Parameterized {
            base: "Option".into(),
            parameters: vec![Shape::Scalar(ScalarKind::Unit)],
        };
        let outer = Shape::Parameterized {
            base: "Vec".into(),
            parameters: vec![inner],
        };
        assert_eq!(outer.depth(), 2);
    }

    #[test]
    fn shape_depth_within_limit() {
        let mut s = Shape::Scalar(ScalarKind::Unit);
        for _ in 0..10 {
            s = Shape::Record(vec![s]);
        }
        assert!(s.is_depth_valid());
    }

    #[test]
    fn shape_depth_beyond_limit() {
        let mut s = Shape::Scalar(ScalarKind::Unit);
        for _ in 0..(MAX_SHAPE_DEPTH + 1) {
            s = Shape::Record(vec![s]);
        }
        assert!(!s.is_depth_valid());
    }

    // --- Shape canonicalization (decision: docs/decisions/shape-canonicalization.md) ---

    /// Two union arms whose compact JSON serializations sort in a known order:
    /// `"scalar"` < `{"record":[...]}` lexicographically.
    fn reorder_union_pair() -> (Shape, Shape) {
        let a = Shape::Record(vec![
            Shape::Scalar(ScalarKind::Unit),
            Shape::Scalar(ScalarKind::Unit),
        ]);
        let b = Shape::Scalar(ScalarKind::Unit);
        (
            Shape::Union(vec![a.clone(), b.clone()]),
            Shape::Union(vec![b, a]),
        )
    }

    #[test]
    fn normalize_leaf_idempotent() {
        assert_eq!(
            Shape::Scalar(ScalarKind::Unit).normalize(),
            Shape::Scalar(ScalarKind::Unit)
        );
        assert_eq!(Shape::Opaque.normalize(), Shape::Opaque);
        assert_eq!(Shape::Bottom.normalize(), Shape::Bottom);
    }

    #[test]
    fn normalize_sorts_union_arms() {
        let (u, _) = reorder_union_pair();
        // After normalization, the lexicographically-smaller arm (Scalar)
        // precedes the Record arm.
        let n = u.normalize();
        let Shape::Union(arms) = &n else {
            panic!("expected union, got {n:?}");
        };
        assert_eq!(
            arms[0],
            Shape::Record(vec![
                Shape::Scalar(ScalarKind::Unit),
                Shape::Scalar(ScalarKind::Unit)
            ])
        );
        assert_eq!(arms[1], Shape::Scalar(ScalarKind::Unit));
    }

    #[test]
    fn normalize_union_equal_under_reorder() {
        let (u1, u2) = reorder_union_pair();
        assert_eq!(u1.normalize(), u2.normalize());
    }

    #[test]
    fn normalize_union_idempotent() {
        let (u, _) = reorder_union_pair();
        let once = u.normalize();
        let twice = once.clone().normalize();
        assert_eq!(once, twice);
    }

    #[test]
    fn normalize_sorts_record_fields() {
        // Record elements are structural (no field names); normalization sorts
        // them by canonical serialization so record order is irrelevant.
        let (a, b) = (
            Shape::Scalar(ScalarKind::Unit),
            Shape::Record(vec![Shape::Scalar(ScalarKind::Unit)]),
        );
        let r1 = Shape::Record(vec![a.clone(), b.clone()]);
        let r2 = Shape::Record(vec![b, a]);
        assert_eq!(r1.normalize(), r2.normalize());
    }

    #[test]
    fn normalize_function_preserves_dom_cod() {
        let (u, _) = reorder_union_pair();
        let f = Shape::Function(Box::new(u.clone()), Box::new(Shape::Opaque));
        let n = f.normalize();
        let Shape::Function(dom, cod) = &n else {
            panic!("expected function, got {n:?}");
        };
        assert_eq!(**dom, u.normalize());
        assert_eq!(**cod, Shape::Opaque);
    }

    #[test]
    fn normalize_ref_inner() {
        let (u, _) = reorder_union_pair();
        let r = Shape::Ref(Box::new(u.clone()));
        let n = r.normalize();
        let Shape::Ref(inner) = &n else {
            panic!("expected ref, got {n:?}");
        };
        assert_eq!(**inner, u.normalize());
    }

    #[test]
    fn normalize_parameterized_preserves_param_order() {
        // Positional sums (e.g. Result<Ok, Err>) are modeled as Parameterized;
        // arm order is significant and MUST be preserved, not sorted.
        let p = Shape::Parameterized {
            base: "Result".into(),
            parameters: vec![Shape::Scalar(ScalarKind::Unit), Shape::Opaque],
        };
        let n = p.normalize();
        let Shape::Parameterized { base, parameters } = &n else {
            panic!("expected parameterized, got {n:?}");
        };
        assert_eq!(base, "Result");
        assert_eq!(
            parameters,
            &vec![Shape::Scalar(ScalarKind::Unit), Shape::Opaque]
        );
    }

    #[test]
    fn normalize_parameterized_normalizes_params() {
        let (u, _) = reorder_union_pair();
        let p = Shape::Parameterized {
            base: "Vec".into(),
            parameters: vec![u.clone()],
        };
        let n = p.normalize();
        let Shape::Parameterized { parameters, .. } = &n else {
            panic!("expected parameterized, got {n:?}");
        };
        assert_eq!(parameters[0], u.normalize());
    }

    #[test]
    fn normalize_nested_recursive() {
        let inner = Shape::Union(vec![Shape::Opaque, Shape::Scalar(ScalarKind::Unit)]);
        let outer = Shape::Record(vec![inner, Shape::Scalar(ScalarKind::Unit)]);
        let n = outer.normalize();
        // Inner union arms sorted by canonical JSON ("opaque" < "scalar"),
        // then record fields sorted ("opaque.." union JSON vs "scalar").
        let expected_inner = Shape::Union(vec![Shape::Opaque, Shape::Scalar(ScalarKind::Unit)]);
        let expected = Shape::Record(vec![Shape::Scalar(ScalarKind::Unit), expected_inner]);
        assert_eq!(n, expected);
    }

    #[test]
    fn normalize_deeply_idempotent() {
        let s = Shape::Union(vec![
            Shape::Record(vec![
                Shape::Union(vec![Shape::Opaque, Shape::Scalar(ScalarKind::Unit)]),
                Shape::Bottom,
            ]),
            Shape::Scalar(ScalarKind::Unit),
        ]);
        let once = s.normalize();
        let twice = once.clone().normalize();
        assert_eq!(once, twice);
    }

    #[test]
    fn canonical_json_round_trips() {
        let s = Shape::Union(vec![
            Shape::Record(vec![Shape::Scalar(ScalarKind::Unit)]),
            Shape::Scalar(ScalarKind::Unit),
        ]);
        let json = s.to_canonical_json();
        let back: Shape = serde_json::from_str(&json).expect("canonical json must deserialize");
        assert_eq!(back.normalize(), s.normalize());
    }

    #[test]
    fn canonical_hash_is_deterministic() {
        let s = Shape::Record(vec![Shape::Scalar(ScalarKind::Unit), Shape::Opaque]);
        assert_eq!(s.canonical_hash(), s.canonical_hash());
    }

    #[test]
    fn canonical_hash_is_128_bit_hex() {
        let h = Shape::Scalar(ScalarKind::Unit).canonical_hash();
        assert_eq!(h.len(), 32, "128-bit hash = 32 hex chars, got {h}");
        assert!(h.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn canonical_hash_content_sensitive() {
        assert_ne!(
            Shape::Scalar(ScalarKind::Unit).canonical_hash(),
            Shape::Opaque.canonical_hash()
        );
        assert_ne!(
            Shape::Record(vec![Shape::Scalar(ScalarKind::Unit)]).canonical_hash(),
            Shape::Union(vec![Shape::Scalar(ScalarKind::Unit)]).canonical_hash()
        );
    }

    #[test]
    fn canonical_hash_equal_under_reorder() {
        let (u1, u2) = reorder_union_pair();
        assert_eq!(u1.canonical_hash(), u2.canonical_hash());
    }

    #[test]
    fn canonical_hash_uses_normalized_form() {
        // The hash of an unnormalized union equals the hash of its normalized
        // form (proves normalization is applied before hashing).
        let (u, _) = reorder_union_pair();
        assert_eq!(u.canonical_hash(), u.normalize().canonical_hash());
    }
}
