//! 管理目录的分页应答。
//!
//! 分页在面向移动端的 [`Page`] 形状之上附加元数据（`offset`、`total`、`kind`），
//! 并不断裁剪，直到编码后的帧落在 IPC 预算内。

use crate::windows_service_impl::management::record::{CommandError, CommandResult};
use qinglink_protocol::service_ipc::{MAX_PAGE_SIZE, Page};
use serde::Serialize;

/// Reject page parameters the admin UI is not allowed to send.
pub(super) fn check_page(offset: usize, limit: usize) -> std::result::Result<(), CommandError> {
    if !(1..=MAX_PAGE_SIZE).contains(&limit) || offset > 1_000_000 {
        return Err(CommandError::new("BAD_REQUEST", "invalid page bounds"));
    }
    Ok(())
}

pub(super) fn bounded_page<T: Serialize>(
    items: Vec<T>,
    offset: usize,
    more: bool,
) -> CommandResult {
    bounded_page_with_total(items, offset, more, None, None)
}

/// Build a page response, shrinking it until it fits the IPC frame budget.
///
/// `total` and the extra `kind` field are additive metadata for the admin UI;
/// the mobile-facing shape is unchanged when they are absent.
pub(super) fn bounded_page_with_total<T: Serialize>(
    mut items: Vec<T>,
    offset: usize,
    mut more: bool,
    total: Option<usize>,
    kind: Option<&str>,
) -> CommandResult {
    // Leave space for response headers and escaped Unicode. Shrinking pages
    // returns the exact next offset, so no row is silently lost.
    while serde_json::to_vec(&items)
        .map_err(CommandError::internal)?
        .len()
        > 60 * 1024
    {
        if items.len() <= 1 {
            return Err(CommandError::new(
                "INTERNAL",
                "item exceeds the IPC response limit",
            ));
        }
        items.pop();
        more = true;
    }
    let next_offset = more.then_some(offset + items.len());
    let mut page =
        serde_json::to_value(Page { items, next_offset }).map_err(CommandError::internal)?;
    if let Some(object) = page.as_object_mut() {
        object.insert("offset".into(), serde_json::json!(offset));
        if let Some(total) = total {
            object.insert("total".into(), serde_json::json!(total));
        }
        if let Some(kind) = kind {
            object.insert("kind".into(), serde_json::json!(kind));
        }
    }
    Ok(page)
}

#[cfg(test)]
mod tests {
    use super::bounded_page;

    #[test]
    fn escaped_rows_shrink_without_losing_the_next_offset() {
        let rows = vec!["\"".repeat(1024); 100];
        let result = bounded_page(rows, 200, false).expect("page");
        let count = result["items"].as_array().expect("items").len();
        assert!(count < 100);
        assert_eq!(result["next_offset"], 200 + count);
        assert!(serde_json::to_vec(&result).expect("json").len() < 64 * 1024);
    }
}
