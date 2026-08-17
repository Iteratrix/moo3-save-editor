//! Species identities behind the `race1` byte.
//!
//! Every population region stores its species as a single byte (`race1`)
//! plus a sub-race/magnate byte (`race2`). The `race1` values come from
//! `racemodifiers.txt` in the game data. Playable picks are a subset; the
//! rest appear via planetary specials, events, and magnate civilizations.

use std::fmt;

/// A species type as stored in a region's `race1` byte.
///
/// Sub-races (Tachidi, Evon, Psilon, ...) share their parent's `race1` and
/// are distinguished only by `race2`; this enum models the `race1` level.
/// [`Species::Unknown`] keeps the parser forward-compatible with modded or
/// unexpected values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Species {
    Human,
    Imsaeis,
    Silicoid,
    Meklar,
    Trilarian,
    /// The Harvester species: bioharvests (eats) cohabiting populations.
    Ithkul,
    Klackon,
    Sakkra,
    Darlok,
    NonCorporeal,
    Protoplasmic,
    Plant,
    Fungal,
    Avian,
    Gargantua,
    Bulrathi,
    Mrrshan,
    Elerian,
    Gnolam,
    Elder,
    ComBot,
    Unknown(u8),
}

/// All species with known `race1` values, in ID order (for pickers).
pub const KNOWN: [Species; 21] = [
    Species::Human,
    Species::Imsaeis,
    Species::Silicoid,
    Species::Meklar,
    Species::Trilarian,
    Species::Ithkul,
    Species::Klackon,
    Species::Sakkra,
    Species::Darlok,
    Species::NonCorporeal,
    Species::Protoplasmic,
    Species::Plant,
    Species::Fungal,
    Species::Avian,
    Species::Gargantua,
    Species::Bulrathi,
    Species::Mrrshan,
    Species::Elerian,
    Species::Gnolam,
    Species::Elder,
    Species::ComBot,
];

impl From<u8> for Species {
    fn from(race1: u8) -> Self {
        match race1 {
            0 => Self::Human,
            1 => Self::Imsaeis,
            2 => Self::Silicoid,
            3 => Self::Meklar,
            4 => Self::Trilarian,
            5 => Self::Ithkul,
            6 => Self::Klackon,
            7 => Self::Sakkra,
            8 => Self::Darlok,
            9 => Self::NonCorporeal,
            10 => Self::Protoplasmic,
            11 => Self::Plant,
            12 => Self::Fungal,
            13 => Self::Avian,
            14 => Self::Gargantua,
            15 => Self::Bulrathi,
            16 => Self::Mrrshan,
            17 => Self::Elerian,
            18 => Self::Gnolam,
            19 => Self::Elder,
            20 => Self::ComBot,
            other => Self::Unknown(other),
        }
    }
}

impl Species {
    /// The `race1` byte written into a region record.
    #[must_use]
    pub fn race1(self) -> u8 {
        match self {
            Self::Human => 0,
            Self::Imsaeis => 1,
            Self::Silicoid => 2,
            Self::Meklar => 3,
            Self::Trilarian => 4,
            Self::Ithkul => 5,
            Self::Klackon => 6,
            Self::Sakkra => 7,
            Self::Darlok => 8,
            Self::NonCorporeal => 9,
            Self::Protoplasmic => 10,
            Self::Plant => 11,
            Self::Fungal => 12,
            Self::Avian => 13,
            Self::Gargantua => 14,
            Self::Bulrathi => 15,
            Self::Mrrshan => 16,
            Self::Elerian => 17,
            Self::Gnolam => 18,
            Self::Elder => 19,
            Self::ComBot => 20,
            Self::Unknown(other) => other,
        }
    }

    /// Look up a species by its canonical name, case-insensitively.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        KNOWN
            .into_iter()
            .find(|species| species.to_string().eq_ignore_ascii_case(name))
    }

    /// Look up a species by canonical name *or* sub-race name.
    ///
    /// Empire records store the sub-race the player picked (e.g. "Tachidi"),
    /// not the `race1`-level species; this maps both to the parent species.
    #[must_use]
    pub fn from_name_or_subrace(name: &str) -> Option<Self> {
        if let Some(species) = Self::from_name(name) {
            return Some(species);
        }
        let subrace = match name.to_ascii_lowercase().as_str() {
            "evon" | "psilon" => Self::Human,
            "eoladi" => Self::Imsaeis,
            "cynoid" => Self::Meklar,
            "nommo" => Self::Trilarian,
            "tachidi" => Self::Klackon,
            "raas" | "grendarl" => Self::Sakkra,
            "alkari" => Self::Avian,
            _ => return None,
        };
        Some(subrace)
    }
}

impl fmt::Display for Species {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Human => write!(f, "Human"),
            Self::Imsaeis => write!(f, "Imsaeis"),
            Self::Silicoid => write!(f, "Silicoid"),
            Self::Meklar => write!(f, "Meklar"),
            Self::Trilarian => write!(f, "Trilarian"),
            Self::Ithkul => write!(f, "Ithkul"),
            Self::Klackon => write!(f, "Klackon"),
            Self::Sakkra => write!(f, "Sakkra"),
            Self::Darlok => write!(f, "Darlok"),
            Self::NonCorporeal => write!(f, "NonCorporeal"),
            Self::Protoplasmic => write!(f, "Protoplasmic"),
            Self::Plant => write!(f, "Plant"),
            Self::Fungal => write!(f, "Fungal"),
            Self::Avian => write!(f, "Avian"),
            Self::Gargantua => write!(f, "Gargantua"),
            Self::Bulrathi => write!(f, "Bulrathi"),
            Self::Mrrshan => write!(f, "Mrrshan"),
            Self::Elerian => write!(f, "Elerian"),
            Self::Gnolam => write!(f, "Gnolam"),
            Self::Elder => write!(f, "Elder"),
            Self::ComBot => write!(f, "ComBot"),
            Self::Unknown(other) => write!(f, "race1={other}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn race1_roundtrips_for_known_species() {
        for species in KNOWN {
            assert_eq!(Species::from(species.race1()), species);
        }
    }

    #[test]
    fn name_lookup_is_case_insensitive() {
        assert_eq!(Species::from_name("ithkul"), Some(Species::Ithkul));
        assert_eq!(Species::from_name("KLACKON"), Some(Species::Klackon));
        assert_eq!(Species::from_name("nope"), None);
    }

    #[test]
    fn subrace_lookup_maps_to_parent() {
        assert_eq!(
            Species::from_name_or_subrace("Tachidi"),
            Some(Species::Klackon)
        );
        assert_eq!(
            Species::from_name_or_subrace("Grendarl"),
            Some(Species::Sakkra)
        );
    }
}
