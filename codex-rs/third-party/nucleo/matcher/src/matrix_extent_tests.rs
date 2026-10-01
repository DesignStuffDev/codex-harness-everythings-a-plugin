//! Inspect raw metadata before forming a view, so this regression can safely
//! demonstrate an invalid extent in the pre-fix implementation.

use super::{MatcherData, MatrixCell, MatrixLayout, MatrixSlab};
use crate::chars::{AsciiChar, Char};
use std::mem::size_of;

fn assert_matrix_extent<C: Char>() {
    let slab = MatrixSlab::new();
    for (haystack_len, needle_len) in [(512, 4), (2048, 2), (512, 128), (1, 1)] {
        let layout = MatrixLayout::<C>::new(haystack_len, needle_len);
        assert!(layout.layout.size() <= size_of::<MatcherData>());
        // Every base.add offset lies within this real allocation. The returned
        // fat pointers remain RAW: never dereference a possibly oversized slice
        // or manufacture a reference just to inspect its pointer metadata.
        let (_, _, _, _, matrix) = unsafe { layout.fieds_from_ptr(slab.0) };
        let expected_len = (haystack_len + 1 - needle_len) * needle_len;
        let matrix_end = matrix.cast::<u8>().addr() + matrix.len() * size_of::<MatrixCell>();
        let slab_end = slab.0.as_ptr().addr() + size_of::<MatcherData>();
        assert_eq!(
            (matrix.len(), matrix_end <= slab_end),
            (expected_len, true),
            "haystack={haystack_len}, needle={needle_len}"
        );
    }
}

#[test]
fn ascii_matrix_view_extent_matches_reserved_layout() {
    assert_matrix_extent::<AsciiChar>();
}

#[test]
fn unicode_matrix_view_extent_matches_reserved_layout() {
    assert_matrix_extent::<char>();
}
