/// How versioned structs handle stored fields absent from the current schema.
/// Fields introduced after the current schema version are silently skipped.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum DeletedFieldPolicy {
    /// Emit a warning and discard the field (the default).
    #[default]
    Warn,
    /// Return `ArchiveError::InvalidData` instead of decoding the value.
    Error,
    /// Panic immediately. Only use when schema mismatches should panic.
    Panic,
}

#[derive(Clone, Copy, Default)]
pub(crate) struct DecodeOptions {
    pub deleted_fields: DeletedFieldPolicy,
    pub warning_handler: Option<fn(&str)>,
}

impl DecodeOptions {
    pub fn deleted_field(self, message: String) -> crate::ArchiveResult<()> {
        match self.deleted_fields {
            DeletedFieldPolicy::Warn => {
                if let Some(handler) = self.warning_handler {
                    handler(&message);
                } else {
                    use std::io::Write;
                    // A closed stderr must not turn the warning policy into a panic.
                    let _ = writeln!(
                        std::io::stderr().lock(),
                        "binary_archive warning: {message}"
                    );
                }
                Ok(())
            }
            DeletedFieldPolicy::Error => Err(crate::ArchiveError::InvalidData(message)),
            DeletedFieldPolicy::Panic => panic!("binary_archive: {message}"),
        }
    }
}
