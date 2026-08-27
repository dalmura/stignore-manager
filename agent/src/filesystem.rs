use std::fs;
use std::path::{Path, PathBuf};
use stignore_lib::ItemGroup;

/* generic functions - keeping for backward compatibility if needed */

/// Checks if a directory entry represents a Syncthing system file, folder, or temporary transfer file
/// These include .stignore, .stfolder, .stversions, any other .st* items, and .syncthing.* temporary files
fn is_syncthing_system_item(entry: &fs::DirEntry) -> bool {
    let name = entry.file_name();
    let name_str = name.to_string_lossy();
    name_str.starts_with(".st") || name_str.starts_with(".syncthing.")
}

/// Recursively calculates total size of a directory in kilobytes
pub fn calculate_dir_size_kb(path: &Path) -> u64 {
    let mut total_bytes = 0u64;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.filter_map(|e| e.ok()) {
            let p = entry.path();
            if let Ok(ft) = entry.file_type() {
                if ft.is_dir() {
                    total_bytes += calculate_dir_size_kb(&p) * 1024;
                } else if let Ok(meta) = entry.metadata() {
                    total_bytes += meta.len();
                }
            }
        }
    }
    total_bytes / 1024
}

#[derive(Debug, Default, Clone)]
pub struct SyncthingMeta {
    pub has_conflicts: bool,
    pub conflict_count: u32,
    pub is_syncing: bool,
    pub stversions_size_kb: u64,
    pub stfolder_present: bool,
}

/// Scans a directory for immediate Syncthing metadata (conflicts, active syncing, .stversions, .stfolder)
pub fn scan_syncthing_meta(dir_path: &Path) -> SyncthingMeta {
    let mut conflict_count = 0u32;
    let mut is_syncing = false;
    let mut stfolder_present = false;
    let mut stversions_size_kb = 0u64;

    if let Ok(entries) = fs::read_dir(dir_path) {
        for entry in entries.filter_map(|e| e.ok()) {
            let name = entry.file_name().to_string_lossy().to_string();

            if name == ".stfolder" {
                stfolder_present = true;
            } else if name == ".stversions" {
                stversions_size_kb = calculate_dir_size_kb(&entry.path());
            }

            if name.starts_with(".syncthing.") {
                is_syncing = true;
            }

            if name.contains(".sync-conflict-") {
                conflict_count += 1;
            }
        }
    }

    SyncthingMeta {
        has_conflicts: conflict_count > 0,
        conflict_count,
        is_syncing,
        stversions_size_kb,
        stfolder_present,
    }
}

/// Constructs a directory ItemGroup combining local Syncthing metadata with aggregated child metadata
pub fn create_dir_item_group(
    id: String,
    name: String,
    dir_path: &Path,
    children: Vec<ItemGroup>,
    leaf: bool,
) -> ItemGroup {
    let meta = scan_syncthing_meta(dir_path);

    let child_conflicts_count: u32 = children.iter().map(|c| c.conflict_count).sum();
    let total_conflict_count = if leaf {
        child_conflicts_count
    } else {
        meta.conflict_count + child_conflicts_count
    };
    let has_conflicts =
        total_conflict_count > 0 || meta.has_conflicts || children.iter().any(|c| c.has_conflicts);

    let child_syncing = children.iter().any(|c| c.is_syncing);
    let is_syncing = meta.is_syncing || child_syncing;

    let child_stversions: u64 = children.iter().map(|c| c.stversions_size_kb).sum();
    let total_stversions = meta.stversions_size_kb + child_stversions;

    let child_stfolder = children.iter().any(|c| c.stfolder_present);
    let stfolder_present = meta.stfolder_present || child_stfolder;

    ItemGroup {
        id,
        name,
        size_kb: children.iter().map(|c| c.size_kb).sum(),
        items: children,
        leaf,
        copy_count: 1,
        has_conflicts,
        conflict_count: total_conflict_count,
        is_syncing,
        stversions_size_kb: total_stversions,
        stfolder_present,
    }
}

/// Helper function to convert folder path components to a full filesystem path
fn build_full_path(base_path: &Path, folder_path_components: &[String]) -> PathBuf {
    let mut full_path = base_path.to_path_buf();
    for component in folder_path_components {
        full_path = full_path.join(component);
    }
    full_path
}

/// Helper function to convert folder path components to Unix-style string for .stignore
fn build_unix_path_string(folder_path_components: &[String]) -> String {
    if folder_path_components.is_empty() {
        "".to_string()
    } else {
        folder_path_components.join("/")
    }
}

fn dir_to_item(entry: fs::DirEntry) -> ItemGroup {
    let filename = entry.file_name().to_string_lossy().to_string();
    let entry_path = entry.path();

    let mut children = build_items(&entry_path, false);
    let mut leaf = false;

    if children.is_empty() {
        children = build_items(&entry_path, true);
        leaf = true;
    }

    create_dir_item_group(filename.clone(), filename, &entry_path, children, leaf)
}

fn file_to_item(entry: fs::DirEntry) -> ItemGroup {
    let filename = entry.file_name().to_string_lossy().to_string();
    let is_conflict = filename.contains(".sync-conflict-");
    let is_syncing = filename.starts_with(".syncthing.");

    ItemGroup {
        id: filename.clone(),
        name: filename,
        size_kb: entry.metadata().map(|m| m.len() / 1024).unwrap_or(0),
        items: vec![],
        leaf: false,
        copy_count: 1,
        has_conflicts: is_conflict,
        conflict_count: if is_conflict { 1 } else { 0 },
        is_syncing,
        stversions_size_kb: 0,
        stfolder_present: false,
    }
}

pub fn build_items(item_path: &Path, leaf: bool) -> Vec<ItemGroup> {
    match fs::read_dir(item_path) {
        Ok(paths) => match leaf {
            true => paths
                .filter_map(|entry| entry.ok())
                .filter(|entry| !is_syncthing_system_item(entry))
                .map(file_to_item)
                .collect(),
            false => paths
                .filter_map(|entry| entry.ok())
                .filter(|entry| !is_syncthing_system_item(entry))
                .filter(|entry| entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
                .map(dir_to_item)
                .collect(),
        },
        Err(why) => {
            tracing::warn!("Unable to list path: {:?}", why.kind());
            vec![]
        }
    }
}

pub fn get_item(start: &Path, path: &[&str]) -> Option<ItemGroup> {
    if path.is_empty() {
        return None;
    }

    let item_name = path[0];
    let children = build_items(start, false);
    let found = children
        .iter()
        .find(|child| child.name == item_name)
        .map(|c| c.to_owned());

    match path.len() {
        1 => found,
        _ => match found {
            Some(child) => {
                let start_here = start.join(&child.name);
                get_item(start_here.as_path(), &path[1..])
            }
            None => None,
        },
    }
}

/// Result of adding a path to .stignore file
#[derive(Debug, Clone)]
pub enum StignoreResult {
    Success {
        ignored_path: String,
        message: String,
    },
    AlreadyIgnored {
        ignored_path: String,
    },
    Error {
        message: String,
    },
}

/// Result of deleting a path from filesystem
#[derive(Debug, Clone)]
pub enum DeleteResult {
    Success {
        deleted_path: String,
        message: String,
    },
    NotFound {
        requested_path: String,
    },
    Error {
        message: String,
    },
}

/// Checks if a folder path is ignored in the .stignore file.
/// This function works with folder path components and supports non-existent folders.
///
/// # Parameters
/// * `category_base_path` - The base directory of the category (e.g., "/home/user/media/movies")
/// * `folder_path_components` - The folder path as components (e.g., ["Movie Name (2023)"])
///
/// # Returns
/// * `bool` - True if the folder path is ignored, false otherwise
pub fn is_path_ignored(
    category_base_path: &std::path::Path,
    folder_path_components: &[String],
) -> bool {
    let folder_path_str = build_unix_path_string(folder_path_components);
    is_path_ignored_str(category_base_path, &folder_path_str)
}

/// Internal helper that works with path strings
fn is_path_ignored_str(category_base_path: &std::path::Path, folder_path: &str) -> bool {
    let stignore_path = category_base_path.join(".stignore");

    // Read .stignore file if it exists
    let ignore_content = match std::fs::read_to_string(&stignore_path) {
        Ok(content) => content,
        Err(_) => return false, // No .stignore file means nothing is ignored
    };

    // Check if the path is in the ignore list
    ignore_content
        .lines()
        .any(|line| line.trim() == folder_path)
}

/// Adds a folder path to the .stignore file in the specified category directory.
/// This function works with folder path components and supports non-existent folders.
///
/// # Parameters
/// * `category_base_path` - The base directory of the category (e.g., "/home/user/media/movies")
/// * `folder_path_components` - The folder path as components (e.g., ["Movie Name (2023)"])
/// * `category_name` - Name of the category for success messages
///
/// # Returns
/// * `StignoreResult` - Success, already ignored, or error result
pub fn add_to_stignore(
    category_base_path: &std::path::Path,
    folder_path_components: &[String],
    category_name: &str,
) -> StignoreResult {
    let folder_path_str = build_unix_path_string(folder_path_components);
    add_to_stignore_str(category_base_path, &folder_path_str, category_name)
}

/// Internal helper that works with path strings
fn add_to_stignore_str(
    category_base_path: &std::path::Path,
    folder_path: &str,
    category_name: &str,
) -> StignoreResult {
    let stignore_path = category_base_path.join(".stignore");

    // Read existing .stignore or handle new file creation
    let mut ignore_content = if stignore_path.exists() {
        match std::fs::read_to_string(&stignore_path) {
            Ok(content) => {
                tracing::debug!(
                    "Successfully read .stignore file with {} bytes",
                    content.len()
                );
                content
            }
            Err(err) => {
                tracing::error!(
                    "Failed to read existing .stignore file at {:?}: {}",
                    stignore_path,
                    err
                );
                return StignoreResult::Error {
                    message: format!(
                        "Failed to read existing .stignore file: {}. This may indicate a file encoding issue or permission problem. Please check the file manually to prevent data loss.",
                        err
                    ),
                };
            }
        }
    } else {
        tracing::info!("Creating new .stignore file at {:?}", stignore_path);
        String::new()
    };

    // Check if the path is already ignored
    if ignore_content
        .lines()
        .any(|line| line.trim() == folder_path)
    {
        tracing::debug!("Path '{}' is already in .stignore", folder_path);
        return StignoreResult::AlreadyIgnored {
            ignored_path: folder_path.to_string(),
        };
    }

    // Add the path to ignore content
    if !ignore_content.is_empty() && !ignore_content.ends_with('\n') {
        ignore_content.push('\n');
    }
    ignore_content.push_str(folder_path);
    ignore_content.push('\n');

    // Create timestamped backup before modifying .stignore
    create_stignore_backup(category_base_path);

    // Write back to .stignore atomically
    match write_stignore_atomic(category_base_path, &ignore_content) {
        Ok(_) => {
            tracing::info!(
                "Successfully added '{}' to .stignore in category '{}'",
                folder_path,
                category_name
            );
            StignoreResult::Success {
                ignored_path: folder_path.to_string(),
                message: format!(
                    "Successfully added '{}' to .stignore in category '{}'",
                    folder_path, category_name
                ),
            }
        }
        Err(err) => {
            tracing::error!(
                "Failed to write .stignore file at {:?}: {}",
                stignore_path,
                err
            );
            StignoreResult::Error {
                message: format!("Failed to write .stignore file: {}", err),
            }
        }
    }
}

/// Removes a folder path from the .stignore file in the specified category directory.
pub fn remove_from_stignore(
    category_base_path: &std::path::Path,
    folder_path_components: &[String],
    category_name: &str,
) -> StignoreResult {
    let folder_path_str = build_unix_path_string(folder_path_components);
    remove_from_stignore_str(category_base_path, &folder_path_str, category_name)
}

fn remove_from_stignore_str(
    category_base_path: &std::path::Path,
    folder_path: &str,
    category_name: &str,
) -> StignoreResult {
    let stignore_path = category_base_path.join(".stignore");

    if !stignore_path.exists() {
        return StignoreResult::Success {
            ignored_path: folder_path.to_string(),
            message: format!("Path '{}' was not present in .stignore", folder_path),
        };
    }

    let ignore_content = match std::fs::read_to_string(&stignore_path) {
        Ok(content) => content,
        Err(err) => {
            return StignoreResult::Error {
                message: format!("Failed to read .stignore file: {}", err),
            };
        }
    };

    let was_present = ignore_content
        .lines()
        .any(|line| line.trim() == folder_path);

    if !was_present {
        return StignoreResult::Success {
            ignored_path: folder_path.to_string(),
            message: format!("Path '{}' was not present in .stignore", folder_path),
        };
    }

    let remaining_lines: Vec<&str> = ignore_content
        .lines()
        .filter(|line| line.trim() != folder_path)
        .collect();

    let new_content = if remaining_lines.is_empty() {
        String::new()
    } else {
        format!("{}\n", remaining_lines.join("\n"))
    };

    // Create timestamped backup before modifying .stignore
    create_stignore_backup(category_base_path);

    match write_stignore_atomic(category_base_path, &new_content) {
        Ok(_) => StignoreResult::Success {
            ignored_path: folder_path.to_string(),
            message: format!(
                "Successfully removed '{}' from .stignore in category '{}'",
                folder_path, category_name
            ),
        },
        Err(err) => StignoreResult::Error {
            message: format!("Failed to write updated .stignore file: {}", err),
        },
    }
}

/// Deletes a folder path from the filesystem in the specified category directory.
/// This function works with folder path components.
///
/// # Parameters
/// * `category_base_path` - The base directory of the category (e.g., "/home/user/media/movies")
/// * `folder_path_components` - The folder path as components (e.g., ["Movie Name (2023)"])
/// * `category_name` - Name of the category for success messages
///
/// # Returns
/// * `DeleteResult` - Success, not found, or error result
pub fn delete_from_filesystem(
    category_base_path: &std::path::Path,
    folder_path_components: &[String],
    category_name: &str,
) -> DeleteResult {
    let full_path = build_full_path(category_base_path, folder_path_components);
    let normalized_folder_path = build_unix_path_string(folder_path_components);

    // Check if the path exists
    if !full_path.exists() {
        return DeleteResult::NotFound {
            requested_path: normalized_folder_path,
        };
    }

    // Attempt to delete the path
    let result = if full_path.is_dir() {
        std::fs::remove_dir_all(&full_path)
    } else {
        std::fs::remove_file(&full_path)
    };

    match result {
        Ok(_) => DeleteResult::Success {
            deleted_path: normalized_folder_path.clone(),
            message: format!(
                "Successfully deleted '{}' from category '{}'",
                normalized_folder_path, category_name
            ),
        },
        Err(err) => DeleteResult::Error {
            message: format!("Failed to delete '{}': {}", normalized_folder_path, err),
        },
    }
}

/// Lists all available .stignore backup files for a category
pub fn list_stignore_backups(
    category_base_path: &std::path::Path,
) -> Vec<stignore_lib::StignoreBackupInfo> {
    let mut backups = Vec::new();

    if let Ok(entries) = std::fs::read_dir(category_base_path) {
        for entry in entries.filter_map(|e| e.ok()) {
            let filename = entry.file_name().to_string_lossy().to_string();
            if let Some(ts_str) = filename.strip_prefix(".stignore.bak.") {
                let metadata = entry.metadata().ok();
                let size_bytes = metadata.as_ref().map(|m| m.len()).unwrap_or(0);

                let timestamp = ts_str.parse::<u64>().unwrap_or_else(|_| {
                    metadata
                        .and_then(|m| m.modified().ok())
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs())
                        .unwrap_or(0)
                });

                let content =
                    std::fs::read_to_string(category_base_path.join(&filename)).unwrap_or_default();

                backups.push(stignore_lib::StignoreBackupInfo {
                    filename,
                    timestamp,
                    size_bytes,
                    content,
                });
            }
        }
    }

    // Sort descending by timestamp (newest first)
    backups.sort_by_key(|b| std::cmp::Reverse(b.timestamp));
    backups
}

/// Prunes old backups keeping at most `max_keep` files
pub fn prune_old_backups(category_base_path: &std::path::Path, max_keep: usize) {
    let mut backups = list_stignore_backups(category_base_path);
    if backups.len() > max_keep {
        for old in backups.drain(max_keep..) {
            let _ = std::fs::remove_file(category_base_path.join(&old.filename));
        }
    }
}

/// Reads the entire .stignore content, current hash, and available backups
pub fn get_stignore_full(
    category_base_path: &std::path::Path,
) -> Result<(String, String, bool, Vec<stignore_lib::StignoreBackupInfo>), String> {
    let stignore_path = category_base_path.join(".stignore");
    let (content, exists) = if stignore_path.exists() {
        match std::fs::read_to_string(&stignore_path) {
            Ok(c) => (c, true),
            Err(e) => return Err(format!("Failed to read .stignore: {}", e)),
        }
    } else {
        (String::new(), false)
    };

    let hash = stignore_lib::compute_content_hash(&content);
    let backups = list_stignore_backups(category_base_path);

    Ok((content, hash, exists, backups))
}

/// Creates a timestamped backup before modifying .stignore and prunes older backups
pub fn create_stignore_backup(category_base_path: &std::path::Path) -> Option<String> {
    let stignore_path = category_base_path.join(".stignore");
    if stignore_path.exists() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let backup_name = format!(".stignore.bak.{}", now);
        let backup_path = category_base_path.join(&backup_name);

        if let Err(e) = std::fs::copy(&stignore_path, &backup_path) {
            tracing::warn!(
                "Failed to create timestamped backup at {:?}: {}",
                backup_path,
                e
            );
            None
        } else {
            prune_old_backups(category_base_path, 10);
            Some(backup_name)
        }
    } else {
        None
    }
}

/// Atomically writes content to .stignore using a temporary file and rename
pub fn write_stignore_atomic(
    category_base_path: &std::path::Path,
    content: &str,
) -> Result<(), String> {
    let stignore_path = category_base_path.join(".stignore");
    let tmp_name = format!(
        ".stignore.tmp.{}.{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    );
    let tmp_path = category_base_path.join(tmp_name);

    if let Err(e) = std::fs::write(&tmp_path, content) {
        return Err(format!("Failed to write temporary .stignore file: {}", e));
    }

    if let Err(e) = std::fs::rename(&tmp_path, &stignore_path) {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(format!("Failed to atomically replace .stignore: {}", e));
    }

    Ok(())
}

/// Saves the full .stignore file with optimistic locking check, backup creation, and atomic file write
pub fn set_stignore_full(
    category_base_path: &std::path::Path,
    content: &str,
    expected_hash: Option<&str>,
    category_name: &str,
) -> Result<(String, Option<String>), String> {
    let stignore_path = category_base_path.join(".stignore");

    // Check optimistic concurrency if file exists
    let mut backup_created = None;
    if stignore_path.exists() {
        let existing_content = match std::fs::read_to_string(&stignore_path) {
            Ok(c) => c,
            Err(e) => return Err(format!("Failed to read existing .stignore: {}", e)),
        };

        let current_hash = stignore_lib::compute_content_hash(&existing_content);
        if let Some(exp) = expected_hash
            && exp != current_hash
        {
            return Err(format!(
                "Conflict: .stignore in category '{}' was modified on disk (current hash: {}, expected: {}). Please reload before saving.",
                category_name, current_hash, exp
            ));
        }

        // Only create backup if content is actually changing
        if existing_content.trim() != content.trim() {
            backup_created = create_stignore_backup(category_base_path);
        }
    }

    // Ensure normalized trailing newline
    let mut normalized_content = content.to_string();
    if !normalized_content.is_empty() && !normalized_content.ends_with('\n') {
        normalized_content.push('\n');
    }

    write_stignore_atomic(category_base_path, &normalized_content)?;

    let new_hash = stignore_lib::compute_content_hash(&normalized_content);
    Ok((new_hash, backup_created))
}

/// Restores an .stignore file from a specific backup
pub fn restore_stignore_backup(
    category_base_path: &std::path::Path,
    backup_filename: &str,
    category_name: &str,
) -> Result<(String, String), String> {
    // Security check: validate filename doesn't contain path traversal
    if backup_filename.contains('/')
        || backup_filename.contains('\\')
        || backup_filename.contains("..")
        || !backup_filename.starts_with(".stignore.bak")
    {
        return Err("Invalid backup filename".to_string());
    }

    let backup_path = category_base_path.join(backup_filename);
    if !backup_path.exists() {
        return Err(format!("Backup file '{}' not found", backup_filename));
    }

    let backup_content = match std::fs::read_to_string(&backup_path) {
        Ok(c) => c,
        Err(e) => return Err(format!("Failed to read backup file: {}", e)),
    };

    let stignore_path = category_base_path.join(".stignore");

    // Create a safety backup of the current file before restoring
    if stignore_path.exists() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let pre_restore_backup = format!(".stignore.bak.pre_restore.{}", now);
        let _ = std::fs::copy(&stignore_path, category_base_path.join(pre_restore_backup));
    }

    // Atomic write
    let tmp_name = format!(
        ".stignore.tmp.restore.{}.{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    );
    let tmp_path = category_base_path.join(tmp_name);

    if let Err(e) = std::fs::write(&tmp_path, &backup_content) {
        return Err(format!("Failed to write temporary restore file: {}", e));
    }

    if let Err(e) = std::fs::rename(&tmp_path, &stignore_path) {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(format!(
            "Failed to atomically restore .stignore in '{}': {}",
            category_name, e
        ));
    }

    let new_hash = stignore_lib::compute_content_hash(&backup_content);
    Ok((backup_content, new_hash))
}
