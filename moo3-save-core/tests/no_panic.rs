use std::io::Read as _;

use moo3_save_core::galaxy::Galaxy;
use moo3_save_core::{empire, verify};
use proptest::prelude::*;

proptest! {
    #[test]
    fn parse_never_panics_on_random_bytes(bytes in proptest::collection::vec(any::<u8>(), 0..4096)) {
        let _ = Galaxy::parse(&bytes);
    }

    #[test]
    fn empires_never_panic_on_random_bytes(bytes in proptest::collection::vec(any::<u8>(), 0..4096)) {
        let _ = empire::empires(&bytes);
    }

    #[test]
    fn verify_never_panics_on_marker_prefixed_bytes(bytes in proptest::collection::vec(any::<u8>(), 0..4096)) {
        let mut data = b"VSYXALAG".to_vec();
        data.extend(&bytes);
        let _ = verify::verify(&data);
    }
}

fn fixture() -> Vec<u8> {
    let path = format!(
        "{}/../test-data/synthesis-turn115.gam.gz",
        env!("CARGO_MANIFEST_DIR")
    );
    let compressed = std::fs::read(path).expect("fixture present");
    let mut bytes = Vec::new();
    flate2::read::GzDecoder::new(compressed.as_slice())
        .read_to_end(&mut bytes)
        .expect("fixture inflates");
    bytes
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(16))]
    #[test]
    fn parse_never_panics_on_mutated_fixture(
        offsets in proptest::collection::vec(0usize..6_800_000, 1..16),
        values in proptest::collection::vec(any::<u8>(), 16),
    ) {
        let mut bytes = fixture();
        for (index, offset) in offsets.iter().enumerate() {
            if *offset < bytes.len() {
                bytes[*offset] = values[index % values.len()];
            }
        }
        if let Ok(galaxy) = Galaxy::parse(&bytes) {
            let _ = empire::player_systems(&bytes, &galaxy);
        }
    }
}
