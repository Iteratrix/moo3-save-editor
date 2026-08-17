/// Errors produced while parsing a save file.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The galaxy marker was not found anywhere in the file.
    ///
    /// Every MOO3 save stores its galaxy section behind the reversed-ASCII
    /// marker `VSYXALAG` ("GALAXYSV"). A file without it is not a MOO3 save,
    /// or is one from a game version with a different layout.
    #[error("galaxy marker VSYXALAG not found (not a MOO3 save?)")]
    NoGalaxyMarker,

    /// A structure declared more data than the file contains.
    ///
    /// The format has no checksums and almost no self-describing lengths, so
    /// a mis-parse usually surfaces as a read past the end of the buffer
    /// rather than a tag mismatch.
    #[error("truncated read: {needed} bytes at offset {offset:#X} (file len {len})")]
    Truncated {
        offset: usize,
        needed: usize,
        len: usize,
    },

    /// A system-name length field was implausible (0 or > 200 characters).
    ///
    /// Name lengths are the format's only cheap sanity anchor: a bad one
    /// means the cursor lost sync somewhere in the previous system's planet
    /// records, and continuing would misattribute every later offset.
    #[error("implausible system name length {len} at offset {offset:#X}")]
    BadSystemName { offset: usize, len: u32 },

    /// The file's header magic `VS3RDAEH` is missing, so header fields
    /// (like the turn counter) cannot be located.
    #[error("header magic VS3RDAEH not found (not a MOO3 save?)")]
    NoHeaderMagic,

    /// The bytes at a region's offset no longer match the parsed record —
    /// the plan is stale or belongs to a different file. Refusing to write
    /// prevents corrupting unrelated bytes.
    #[error(
        "region at offset {offset:#X} does not match the parsed record (stale plan or wrong file)"
    )]
    StaleRegion { offset: usize },

    /// A value cannot be represented in the save's wire format.
    #[error("{what} {value} does not fit the save format's range")]
    ValueOutOfRange { what: &'static str, value: f64 },

    /// An empire's `PLAYERSV` record (which holds its treasury) could not
    /// be located by its name.
    #[error("empire record for \"{name}\" not found in PLAYERSV")]
    EmpireRecordNotFound { name: String },
}

pub type Result<T> = std::result::Result<T, Error>;
