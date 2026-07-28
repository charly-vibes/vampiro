//! Vampiro's managed block configuration.
//!
//! Vampiro registers its own `VAMPIRO` managed block for agent instructions.
//! The injector is sourced from `genesis::managed_block`.

use genesis::managed_block::{BlockDef, BlockInjector, BlockRegistry};

/// Create a `BlockRegistry` with Vampiro's managed block.
///
/// Registers the `VAMPIRO` block for vampiro's own agent instructions.
pub fn vampiro_registry() -> BlockRegistry {
    let mut reg = BlockRegistry::new();
    reg.register(BlockDef::new("VAMPIRO"));
    reg
}

/// Create a `BlockInjector` configured with Vampiro's managed blocks.
pub fn vampiro_injector() -> BlockInjector {
    BlockInjector::new(vampiro_registry())
}
