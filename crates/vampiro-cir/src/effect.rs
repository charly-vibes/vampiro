/// Maximum allowed nesting depth for effect channels.
///
/// This bounds recursive structures like `Recursive(Recursive(...))`
/// to prevent stack overflow or memory exhaustion on comparison/serialization.
pub const MAX_EFFECT_DEPTH: u32 = 64;

/// Built-in effect channel IDs.
///
/// These represent the effect wrapping on a node's output.
/// Project-declared IDs are stored as strings; `Unknown` is the sentinel
/// for unrecognized wrappers — it never defaults to `Plain`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EffectChannel {
    /// No effect — a pure computation.
    Plain,
    /// A value or error (Result-like).
    Result,
    /// A value or absence (Option-like).
    Option,
    /// An effect that may throw an exception.
    Throws,
    /// An asynchronous computation.
    Async,
    /// A stream of values.
    Stream,
    /// A project-declared effect/functor ID.
    Custom(String),
    /// Recursive combination of effect channels.
    Recursive(Box<EffectChannel>),
    /// Sentinel for unrecognized wrappers — never defaults to `Plain`.
    Unknown,
}

impl EffectChannel {
    /// Compute the maximum nesting depth of this effect channel.
    ///
    /// Leaf variants (Plain, Result, Option, Throws, Async, Stream, Custom, Unknown)
    /// have depth 0. Recursive has depth = 1 + depth of inner.
    pub fn depth(&self) -> u32 {
        match self {
            EffectChannel::Recursive(inner) => 1 + inner.depth(),
            _ => 0,
        }
    }

    /// Returns `true` if the effect depth is within the allowed limit.
    pub fn is_depth_valid(&self) -> bool {
        self.depth() <= MAX_EFFECT_DEPTH
    }
}

/// Built-in effect resolution IDs.
///
/// These describe how an edge resolves the effect channel between nodes.
/// Project-declared resolutions are stored as strings.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EffectResolution {
    /// The effect passes through unchanged.
    Propagated,
    /// The effect is transformed to a different channel.
    Transformed,
    /// The wrapper is removed (ordinary unwrap or force/panic unwrap).
    Unwrapped,
    /// The effect is swallowed (panic/force removal with partial totality).
    Swallowed,
    /// The operation is retried on failure.
    Retried,
    /// A project-declared resolution/natural-transformation ID.
    Custom(String),
    /// Sentinel for unrecognized resolutions.
    Unknown,
}

/// Evidence for an unwrap resolution.
///
/// Totality semantics (see design.md for the full 2×2 matrix):
///
/// | kind \ totality | Total | Partial |
/// |-----------------|-------|---------|
/// | Ordinary        | `Unwrapped, Total` — all branches handled (e.g. `?` operator) | `Unwrapped, Partial` — ordinary unwrap with unhandled branches |
/// | Force           | `Swallowed, Total` — every summand has an intentional branch | `Swallowed, Partial` — force/panic unwrap with unhandled branches |
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct UnwrapEvidence {
    /// How the wrapper was removed.
    pub kind: UnwrapKind,
    /// The totality of the unwrap — whether all branches are handled.
    pub totality: Totality,
}

/// How a wrapper was removed.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UnwrapKind {
    /// Ordinary wrapper removal (e.g., `?` operator, `.unwrap()` with match).
    Ordinary,
    /// Panic/force removal (e.g., `.unwrap()` on `None`, forced downcast).
    Force,
}

/// Totality of an unwrap or effect handling.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Totality {
    /// All branches are handled.
    Total,
    /// Some branches may be unhandled.
    Partial,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effect_channel_plain() {
        assert_eq!(
            serde_json::to_value(EffectChannel::Plain).unwrap(),
            serde_json::json!("plain")
        );
    }

    #[test]
    fn effect_channel_recursive() {
        let ch = EffectChannel::Recursive(Box::new(EffectChannel::Option));
        assert_eq!(
            serde_json::to_value(ch).unwrap(),
            serde_json::json!({"recursive": "option"})
        );
    }

    #[test]
    fn effect_channel_custom() {
        let ch = EffectChannel::Custom("my-effect".into());
        assert_eq!(
            serde_json::to_value(ch).unwrap(),
            serde_json::json!({"custom": "my-effect"})
        );
    }

    #[test]
    fn effect_channel_unknown() {
        assert_eq!(
            serde_json::to_value(EffectChannel::Unknown).unwrap(),
            serde_json::json!("unknown")
        );
    }

    #[test]
    fn effect_resolution_propagated() {
        assert_eq!(
            serde_json::to_value(EffectResolution::Propagated).unwrap(),
            serde_json::json!("propagated")
        );
    }

    #[test]
    fn effect_resolution_unwrapped() {
        assert_eq!(
            serde_json::to_value(EffectResolution::Unwrapped).unwrap(),
            serde_json::json!("unwrapped")
        );
    }

    #[test]
    fn unwrap_evidence_serialization() {
        let ev = UnwrapEvidence {
            kind: UnwrapKind::Ordinary,
            totality: Totality::Total,
        };
        let json = serde_json::to_value(ev).unwrap();
        assert_eq!(json["kind"], "ordinary");
        assert_eq!(json["totality"], "total");
    }

    #[test]
    fn effect_depth_plain() {
        assert_eq!(EffectChannel::Plain.depth(), 0);
    }

    #[test]
    fn effect_depth_recursive() {
        let ch = EffectChannel::Recursive(Box::new(EffectChannel::Option));
        assert_eq!(ch.depth(), 1);
    }

    #[test]
    fn effect_depth_deeply_nested() {
        let ch = EffectChannel::Recursive(Box::new(EffectChannel::Recursive(Box::new(
            EffectChannel::Recursive(Box::new(EffectChannel::Result)),
        ))));
        assert_eq!(ch.depth(), 3);
    }

    #[test]
    fn effect_depth_within_limit() {
        let mut ch = EffectChannel::Plain;
        for _ in 0..10 {
            ch = EffectChannel::Recursive(Box::new(ch));
        }
        assert!(ch.is_depth_valid());
    }

    #[test]
    fn effect_depth_beyond_limit() {
        let mut ch = EffectChannel::Plain;
        for _ in 0..(MAX_EFFECT_DEPTH + 1) {
            ch = EffectChannel::Recursive(Box::new(ch));
        }
        assert!(!ch.is_depth_valid());
    }
}
