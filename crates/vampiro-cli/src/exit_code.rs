/// Neutral exit-code types for the Vampiro CLI.
///
/// This enum defines the contract between Vampiro and its callers.
/// It does not depend on any analysis, gating, or proof logic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ExitCode {
    /// Success — all checks passed, or help/version displayed
    Success = 0,
    /// Invalid config — config file not found, unreadable, or malformed
    InvalidConfig = 1,
    /// Usage error — unknown flag, unknown command, missing required argument
    UsageError = 2,
    /// Policy failure — check found violations above the accept threshold
    PolicyFailure = 3,
    /// Internal error — I/O failure, panic, unexpected runtime condition
    InternalError = 4,
}

impl From<ExitCode> for i32 {
    fn from(code: ExitCode) -> i32 {
        code as i32
    }
}

impl std::process::Termination for ExitCode {
    fn report(self) -> std::process::ExitCode {
        std::process::ExitCode::from(self as u8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_code_values_match_contract() {
        assert_eq!(i32::from(ExitCode::Success), 0);
        assert_eq!(i32::from(ExitCode::InvalidConfig), 1);
        assert_eq!(i32::from(ExitCode::UsageError), 2);
        assert_eq!(i32::from(ExitCode::PolicyFailure), 3);
        assert_eq!(i32::from(ExitCode::InternalError), 4);
    }
}
