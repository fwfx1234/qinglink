// === Custom directory responses ===

#[derive(Debug, Serialize)]
pub struct CustomDirListResponse {
    pub dirs: Vec<crate::custom_dir::CustomDir>,
}


