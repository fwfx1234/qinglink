// === Files ===

#[derive(Debug, Clone, Serialize)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: Option<u64>,
    pub is_hidden: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct DirectoryListing {
    pub current_path: String,
    pub parent_path: Option<String>,
    pub entries: Vec<FileEntry>,
    pub total_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileInfo {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: Option<u64>,
    pub created: Option<u64>,
    pub is_hidden: bool,
    pub is_readonly: bool,
    pub extension: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct BrowseQuery {
    pub path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct FilePathQuery {
    pub path: String,
}

#[derive(Debug, Deserialize)]
pub struct RenameFileRequest {
    pub old_path: String,
    pub new_path: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateDirRequest {
    pub path: String,
    pub name: String,
}


