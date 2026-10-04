//! 自定义目录管理器的既有单元测试。

use super::*;
use std::fs;

fn temp_config_path() -> std::path::PathBuf {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test_custom_dirs.json");
    // 防止 tempdir 在测试结束时被删除
    std::mem::forget(dir);
    path
}

#[test]
fn add_and_list_custom_dir() {
    let path = temp_config_path();
    let mut manager = CustomDirManager::new(path.clone());

    let test_dir = tempfile::tempdir().unwrap();
    let test_path = test_dir.path().to_string_lossy().to_string();

    let req = AddCustomDirRequest {
        path: test_path.clone(),
        name: Some("测试目录".to_string()),
        enabled: true,
        max_depth: 3,
        extensions: None,
        recursive: true,
    };

    let dir = manager.add(req).unwrap();
    assert_eq!(dir.path, test_path);
    assert_eq!(dir.name, "测试目录");
    assert!(dir.enabled);
    assert_eq!(dir.max_depth, 3);

    let dirs = manager.list();
    assert_eq!(dirs.len(), 1);
    assert_eq!(dirs[0].path, test_path);

    std::mem::forget(test_dir);
    let _ = fs::remove_file(&path);
}

#[test]
fn add_duplicate_dir_fails() {
    let path = temp_config_path();
    let mut manager = CustomDirManager::new(path.clone());

    let test_dir = tempfile::tempdir().unwrap();
    let test_path = test_dir.path().to_string_lossy().to_string();

    let req = AddCustomDirRequest {
        path: test_path,
        name: None,
        enabled: true,
        max_depth: 0,
        extensions: None,
        recursive: true,
    };

    manager.add(req.clone()).unwrap();
    let result = manager.add(req);
    assert!(result.is_err(), "重复添加应失败");

    std::mem::forget(test_dir);
    let _ = fs::remove_file(&path);
}

#[test]
fn add_nonexistent_path_fails() {
    let path = temp_config_path();
    let mut manager = CustomDirManager::new(path.clone());

    let req = AddCustomDirRequest {
        path: "Z:\\不存在的路径\\测试".to_string(),
        name: None,
        enabled: true,
        max_depth: 0,
        extensions: None,
        recursive: true,
    };

    let result = manager.add(req);
    assert!(result.is_err(), "添加不存在的路径应失败");

    let _ = fs::remove_file(&path);
}

#[test]
fn update_custom_dir() {
    let path = temp_config_path();
    let mut manager = CustomDirManager::new(path.clone());

    let test_dir = tempfile::tempdir().unwrap();
    let test_path = test_dir.path().to_string_lossy().to_string();

    let req = AddCustomDirRequest {
        path: test_path,
        name: Some("原始名称".to_string()),
        enabled: true,
        max_depth: 0,
        extensions: None,
        recursive: true,
    };

    let dir = manager.add(req).unwrap();
    let id = dir.id;

    let update_req = UpdateCustomDirRequest {
        path: None,
        name: Some("新名称".to_string()),
        enabled: Some(false),
        max_depth: Some(5),
        extensions: None,
        recursive: None,
    };

    let updated = manager.update(&id, update_req).unwrap();
    assert_eq!(updated.name, "新名称");
    assert!(!updated.enabled);
    assert_eq!(updated.max_depth, 5);

    std::mem::forget(test_dir);
    let _ = fs::remove_file(&path);
}

#[test]
fn remove_custom_dir() {
    let path = temp_config_path();
    let mut manager = CustomDirManager::new(path.clone());

    let test_dir = tempfile::tempdir().unwrap();
    let test_path = test_dir.path().to_string_lossy().to_string();

    let req = AddCustomDirRequest {
        path: test_path,
        name: None,
        enabled: true,
        max_depth: 0,
        extensions: None,
        recursive: true,
    };

    let dir = manager.add(req).unwrap();
    assert_eq!(manager.list().len(), 1);

    manager.remove(&dir.id).unwrap();
    assert_eq!(manager.list().len(), 0);

    std::mem::forget(test_dir);
    let _ = fs::remove_file(&path);
}

#[test]
fn remove_nonexistent_dir_fails() {
    let path = temp_config_path();
    let mut manager = CustomDirManager::new(path.clone());

    let result = manager.remove("不存在的ID");
    assert!(result.is_err(), "删除不存在的目录应失败");

    let _ = fs::remove_file(&path);
}

#[test]
fn validate_existing_directory() {
    let test_dir = tempfile::tempdir().unwrap();
    let result = CustomDirManager::validate(test_dir.path().to_str().unwrap());

    assert!(result.valid);
    assert!(result.exists);
    assert!(result.is_dir);
    assert!(result.readable);
}

#[test]
fn validate_nonexistent_path() {
    let result = CustomDirManager::validate("Z:\\不存在的路径\\测试");

    assert!(!result.valid);
    assert!(!result.exists);
    assert!(!result.is_dir);
    assert!(!result.readable);
    assert!(result.error.is_some());
}

#[test]
fn validate_file_instead_of_dir() {
    let path = temp_config_path();
    let _ = fs::write(&path, "test");

    let result = CustomDirManager::validate(path.to_str().unwrap());
    assert!(!result.valid);
    assert!(result.exists);
    assert!(!result.is_dir);

    let _ = fs::remove_file(&path);
}

#[test]
fn persistence_across_instances() {
    let path = temp_config_path();

    let test_dir = tempfile::tempdir().unwrap();
    let test_path = test_dir.path().to_string_lossy().to_string();

    {
        let mut manager = CustomDirManager::new(path.clone());
        let req = AddCustomDirRequest {
            path: test_path,
            name: Some("持久化测试".to_string()),
            enabled: true,
            max_depth: 0,
            extensions: None,
            recursive: true,
        };
        manager.add(req).unwrap();
    }

    {
        let manager = CustomDirManager::new(path.clone());
        let dirs = manager.list();
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].name, "持久化测试");
    }

    std::mem::forget(test_dir);
    let _ = fs::remove_file(&path);
}
