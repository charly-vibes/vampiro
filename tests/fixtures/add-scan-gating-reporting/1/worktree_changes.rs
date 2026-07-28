// Fixture: synthetic worktree with staged + unstaged + untracked changes.
// Tests that local_diff captures all three categories.
fn existing() -> u32 { 42 }

// Staged change: was `fn staged_added() -> bool { true }` — not yet in HEAD
pub fn staged_added() -> bool { false }

// Unstaged change: was `fn existing() -> u32 { 42 }` — now modified
// Actually the existing fn above is the unstaged version