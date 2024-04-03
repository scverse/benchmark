/// Finds the closest `x` not exceeding `index` where `is_char_boundary(x)` is `true`.
///
/// This method can help you truncate a string so that it's still valid UTF-8, but doesn't
/// exceed a given number of bytes.
///
/// See <https://doc.rust-lang.org/std/primitive.str.html#method.floor_char_boundary>
#[inline]
pub(crate) fn floor_char_boundary(s: &str, index: usize) -> usize {
    if index >= s.len() {
        s.len()
    } else {
        let lower_bound = index.saturating_sub(3);
        let new_index = s.as_bytes()[lower_bound..=index]
            .iter()
            .rposition(|b| is_utf8_char_boundary(*b));

        // SAFETY: we know that the character boundary will be within four bytes
        unsafe { lower_bound + new_index.unwrap_unchecked() }
    }
}

#[allow(clippy::cast_possible_wrap)]
#[inline]
pub(crate) fn is_utf8_char_boundary(b: u8) -> bool {
    // This is bit magic equivalent to: b < 128 || b >= 192
    (b as i8) >= -0x40
}
