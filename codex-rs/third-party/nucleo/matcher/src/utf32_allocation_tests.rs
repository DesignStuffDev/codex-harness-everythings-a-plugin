use std::alloc::Layout;

use super::{ColumnStorage, Utf32AllocationError, Utf32String};

#[test]
fn planned_conversion_matches_existing_variants_and_actual_payload_sizes() {
    for input in [
        "",
        "src/simple-path.rs",
        "ascii\tand\rcarriage-return\n",
        "line\r\nnext",
        "u\u{0308}/naïve/中.rs",
        "👩\u{200d}💻/🇦🇺.txt",
    ] {
        let expected = Utf32String::from(input);
        let plan = Utf32String::allocation_plan(input).unwrap();
        let charged_bytes = plan.charged_bytes();
        let actual = plan.allocate().unwrap();
        let payload_bytes = match &actual {
            Utf32String::Ascii(value) => Layout::for_value(value.as_ref()).size(),
            Utf32String::Unicode(value) => Layout::for_value(value.as_ref()).size(),
        };
        assert_eq!(actual, expected, "input={input:?}");
        assert_eq!(charged_bytes, payload_bytes, "input={input:?}");
    }
}

#[test]
fn ascii_crlf_retains_unicode_representation_and_feature_specific_conversion() {
    let input = "a\r\nb";
    let expected = if cfg!(feature = "unicode-segmentation") {
        vec!['a', '\n', 'b']
    } else {
        vec!['a', '\r', '\n', 'b']
    };
    let plan = Utf32String::allocation_plan(input).unwrap();
    assert_eq!(
        plan.charged_bytes(),
        expected.len() * std::mem::size_of::<char>()
    );
    assert_eq!(
        plan.allocate().unwrap(),
        Utf32String::Unicode(expected.into_boxed_slice())
    );
}

#[test]
fn combining_graphemes_preserve_segmentation_without_byte_length_overcharging() {
    let input = "u\u{0308}x";
    let expected = if cfg!(feature = "unicode-segmentation") {
        vec!['u', 'x']
    } else {
        vec!['u', '\u{0308}', 'x']
    };
    let plan = Utf32String::allocation_plan(input).unwrap();
    assert_eq!(
        plan.charged_bytes(),
        expected.len() * std::mem::size_of::<char>()
    );
    assert_eq!(
        plan.allocate().unwrap(),
        Utf32String::Unicode(expected.into_boxed_slice())
    );
}

#[test]
fn larger_conversion_keeps_exact_capacity_for_non_power_of_two_lengths() {
    for input in ["a".repeat(1025), "x\r\nu\u{0308}中👩\u{200d}💻".repeat(257)] {
        let expected = Utf32String::from(input.as_str());
        let plan = Utf32String::allocation_plan(&input).unwrap();
        let charged_bytes = plan.charged_bytes();
        let actual = plan.allocate().unwrap();
        let payload_bytes = match &actual {
            Utf32String::Ascii(value) => value.len(),
            Utf32String::Unicode(value) => value.len() * std::mem::size_of::<char>(),
        };
        assert_eq!((actual, charged_bytes), (expected, payload_bytes));
    }
}

#[test]
fn target_layout_overflow_is_rejected_without_constructing_an_oversized_string() {
    let largest_char_count = isize::MAX as usize / std::mem::size_of::<char>();
    assert_eq!(
        ColumnStorage::Unicode(largest_char_count)
            .layout()
            .unwrap()
            .size(),
        largest_char_count * std::mem::size_of::<char>()
    );
    for storage in [
        ColumnStorage::Ascii(isize::MAX as usize + 1),
        ColumnStorage::Unicode(largest_char_count + 1),
        ColumnStorage::Unicode(usize::MAX),
    ] {
        assert_eq!(storage.layout(), Err(Utf32AllocationError::SizeOverflow));
    }
}
