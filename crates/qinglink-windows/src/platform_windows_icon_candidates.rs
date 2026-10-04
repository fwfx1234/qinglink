//! 图标候选枚举：列出用户可为条目选择的所有图标来源，最佳猜测在前。

use super::ComApartment;
use super::icon_resolve::IconCandidateInfo;
use super::icon_shortcut::{
    is_executable, is_shortcut, resolve_lnk_icon_location, sibling_icon_paths,
};
use super::resolve_lnk_target;

fn push_candidate(
    candidates: &mut Vec<IconCandidateInfo>,
    seen: &mut std::collections::HashSet<String>,
    path: &std::path::Path,
    kind: &str,
) {
    if !path.is_file() {
        return;
    }
    let key = path.to_string_lossy().to_lowercase();
    if !seen.insert(key) {
        return;
    }
    candidates.push(IconCandidateInfo {
        path: path.to_string_lossy().to_string(),
        kind: kind.to_string(),
        size_bytes: std::fs::metadata(path).map(|meta| meta.len()).unwrap_or(0),
    });
}

/// Every icon source a user could choose for an entry, best guess first.
pub fn list_icon_candidates(path: &str) -> Vec<IconCandidateInfo> {
    let original = std::path::Path::new(path);
    let mut seen = std::collections::HashSet::new();
    let mut candidates = Vec::new();
    let mut anchor = original.to_path_buf();

    if is_shortcut(original) {
        let _com = ComApartment::init().ok();
        if let Some((location, _index)) = resolve_lnk_icon_location(original) {
            push_candidate(
                &mut candidates,
                &mut seen,
                std::path::Path::new(&location),
                "shortcut_icon",
            );
        }
        let (target, _arguments) = resolve_lnk_target(original);
        let target_path = std::path::PathBuf::from(&target);
        if target_path.is_file() {
            anchor = target_path;
        }
        push_candidate(&mut candidates, &mut seen, original, "shortcut");
    }

    if is_executable(&anchor) {
        push_candidate(&mut candidates, &mut seen, &anchor, "executable");
    }
    for candidate in sibling_icon_paths(&anchor) {
        push_candidate(&mut candidates, &mut seen, &candidate, "image");
    }
    candidates.truncate(40);
    candidates
}
