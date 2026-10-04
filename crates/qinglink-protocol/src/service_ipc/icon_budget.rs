//! 图标预算校验：协议允许传输的最大图标体积，以及编码串的预算判定。

/// Largest decoded icon payload the protocol carries.
///
/// The extractor is expected to stay inside this budget by re-encoding at a
/// smaller edge; the callers only use it as a safety net so a pathological
/// bitmap cannot grow the transport without bound.
pub const MAX_ICON_BYTES: usize = 128 * 1024;

/// Whether a base64 icon payload fits the protocol budget.
///
/// Every icon producer (`icon_service`, the service's management IPC, the
/// scanner) must agree on this check: comparing the *decoded* length in one
/// place and the *encoded* length in another is what let an oversized icon be
/// dropped by one path and served by the next.
///
/// The comparison is made on the decoded size. Base64 only encodes whole
/// 3-byte groups, so a length check cannot tell a payload that sits exactly on
/// the budget from one that is a couple of bytes over it — and that last couple
/// of bytes is exactly the margin a budget has to hold.
pub fn icon_fits_budget(encoded: &str) -> bool {
    decoded_len(encoded).is_some_and(|decoded| decoded <= MAX_ICON_BYTES)
}

/// Size the given standard-base64 string decodes to, without decoding it.
fn decoded_len(encoded: &str) -> Option<usize> {
    let length = encoded.len();
    if length == 0 || !length.is_multiple_of(4) {
        return None;
    }
    let padding = encoded
        .as_bytes()
        .iter()
        .rev()
        .take_while(|byte| **byte == b'=')
        .count();
    (padding <= 2).then(|| length / 4 * 3 - padding)
}

#[cfg(test)]
mod icon_budget_tests {
    use super::{MAX_ICON_BYTES, decoded_len, icon_fits_budget};

    fn encode(bytes: usize) -> String {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD.encode(vec![0_u8; bytes])
    }

    #[test]
    fn accepts_a_payload_exactly_on_the_budget() {
        let encoded = encode(MAX_ICON_BYTES);
        assert_eq!(decoded_len(&encoded), Some(MAX_ICON_BYTES));
        assert!(icon_fits_budget(&encoded));
    }

    #[test]
    fn rejects_a_payload_one_byte_over_the_budget() {
        let encoded = encode(MAX_ICON_BYTES + 1);
        assert_eq!(decoded_len(&encoded), Some(MAX_ICON_BYTES + 1));
        assert!(!icon_fits_budget(&encoded));
    }

    #[test]
    fn rejects_anything_that_is_not_base64() {
        assert!(!icon_fits_budget(""));
        assert!(!icon_fits_budget("not base64"));
        assert!(!icon_fits_budget("===="));
    }
}
