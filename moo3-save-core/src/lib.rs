//! Parser and editor for Master of Orion 3 save files (`.gam`).
//!
//! The format was reverse-engineered from Bhruic's MOO3 Save Editor v0.51
//! and the game binary: big-endian integers, UTF-16BE strings, a custom
//! 6+2-byte fixed-point number, and section markers stored as reversed
//! ASCII (`VSYXALAG` is "GALAXYSV"). Format notes live in each module's
//! docs.
//!
//! Built to solve the Ithkul problem — Harvesters bioharvesting (eating)
//! every population that shares a planet with them, with no in-game remedy —
//! but works as a general species replacement tool.
//!
//! # Layers
//!
//! - [`galaxy`]: systems, planets, and population regions — the parse that
//!   locates every region's editable fields (owner, population, species,
//!   terrain, ecosystem).
//! - [`species`]: the [`Species`] enum behind the `race1` byte.
//! - [`empire`]: the empire table and player-ownership detection.
//! - [`replace`]: plan a species replacement, then patch it in place.
//! - [`edit`]: field-level region edits (population, owner, terrain,
//!   ecosystem).
//! - [`header`]: file-header fields (the turn counter).
//! - [`verify`]: the check battery run by tests and `corpus`.
//!
//! # Example
//!
//! ```no_run
//! use moo3_save_core::galaxy::Galaxy;
//! use moo3_save_core::replace::{self, Scope};
//! use moo3_save_core::Species;
//!
//! let mut bytes = std::fs::read("save.gam")?;
//! let galaxy = Galaxy::parse(&bytes)?;
//! let scope = Scope::Shared { protect: None };
//! let planned = replace::plan(&galaxy, Species::Ithkul, &scope, None);
//! let outcome = replace::apply(&mut bytes, &planned, Species::Ithkul, Species::Klackon);
//! println!("converted {} regions", outcome.patched);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

mod cursor;
pub mod edit;
pub mod empire;
pub mod error;
pub mod galaxy;
pub mod header;
pub mod replace;
mod special;
pub mod species;
pub mod verify;

pub use error::{Error, Result};
pub use species::Species;
