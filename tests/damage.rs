//! Arbitrary bytes, and the test catalog cut anywhere or damaged anywhere,
//! give a catalog with problems or an error: never a panic.

use std::io::Read;

use proptest::prelude::*;

/// The test catalog's page size.
const PAGE_SIZE: usize = 4096;

fn catalog() -> Vec<u8> {
    let compressed = include_bytes!("fixtures/plaso/Catalog1.edb.gz");
    let mut data = Vec::new();
    common::gzip::Decoder::new(&compressed[..])
        .read_to_end(&mut data)
        .unwrap();
    data
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]

    #[test]
    fn arbitrary_bytes(data in proptest::collection::vec(any::<u8>(), 0..16_384)) {
        let _ = filehistory::read(&data);
        let _ = filehistory::detect(&data);
    }

    #[test]
    fn cut_anywhere(at in 0usize..3_200_000) {
        let data = catalog();
        let _ = filehistory::read(&data[..at.min(data.len())]);
    }

    #[test]
    fn cut_at_a_page(pages in 1usize..770) {
        let data = catalog();
        let _ = filehistory::read(&data[..(pages * PAGE_SIZE).min(data.len())]);
    }

    #[test]
    fn damaged(flips in proptest::collection::vec((any::<usize>(), any::<u8>()), 1..200)) {
        let mut data = catalog();
        for (at, byte) in flips {
            let at = at % data.len();
            data[at] = byte;
        }
        let _ = filehistory::read(&data);
    }
}

#[test]
fn not_a_catalog() {
    assert!(filehistory::read(b"").is_err());
    assert!(!filehistory::detect(b"not an ESE database"));
}

#[test]
fn a_cut_catalog_reads_what_it_holds() {
    let data = catalog();
    // The first fifth: all the strings, part of the namespace.
    let cut = filehistory::read(&data[..data.len() / 5]).unwrap();
    assert_eq!(cut.strings.len(), 994);
    assert!(!cut.entries.is_empty() && cut.entries.len() < 1373);
    assert!(cut.problems.iter().any(|p| p.starts_with("namespace: ")));
}
