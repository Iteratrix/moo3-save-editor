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
}

pub type Result<T> = std::result::Result<T, Error>;
