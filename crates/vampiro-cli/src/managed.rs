//! Vampiro's managed block configuration.
//!
//! These blocks are injected into project files for `wai status` detection
//! and other tooling integration. The injector is sourced from
//! `genesis::managed_block`.

use genesis::managed_block::{BlockDef, BlockInjector, BlockRegistry};

/// Create a `BlockRegistry` with Vampiro's three managed blocks.
///
/// Registers `WAI`, `OPENSPEC`, and `DONT` blocks — the three blocks
/// needed for `wai status` detection (`wai-bdqw.9`).
pub fn vampiro_registry() -> BlockRegistry {
    let mut reg = BlockRegistry::new();
    reg.register(BlockDef::new("WAI"));
    reg.register(BlockDef::new("OPENSPEC"));
    reg.register(BlockDef::new("DONT"));
    reg
}

/// Create a `BlockInjector` configured with Vampiro's managed blocks.
pub fn vampiro_injector() -> BlockInjector {
    BlockInjector::new(vampiro_registry())
}
