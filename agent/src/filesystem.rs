use std::fs;
use std::path::{Path, PathBuf};
use stignore_lib::ItemGroup;

/* generic functions - keeping for backward compatibility if needed */

/// Checks if a directory entry represents a Syncthing system file or folder
/// These include .stignore, .stfolder, .stversions, and any other .st* items
fn is_syncthing_system_item(entry: &fs::DirEntry) -> bool {
    entry.file_name().to_string_lossy().starts_with(".st")
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
        "/".to_string()
    } else {
        format!("/{}", folder_path_components.join("/"))
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

    ItemGroup {
        id: filename.clone(),
        name: filename,
        size_kb: children.iter().map(|c| c.size_kb).sum(),
        items: children,
        leaf,
        copy_count: 1,
    }
}

fn file_to_item(entry: fs::DirEntry) -> ItemGroup {
    let filename = entry.file_name().to_string_lossy().to_string();

    ItemGroup {
        id: filename.clone(),
        name: filename,
        size_kb: entry.metadata().map(|m| m.len() / 1024).unwrap_or(0),
        items: vec![],
        leaf: false,
        copy_count: 1,
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

    // Normalize the path to ensure consistency
    let normalized_path = if folder_path.starts_with('/') {
        folder_path.to_string()
    } else {
        format!("/{}", folder_path)
    };

    // Read .stignore file if it exists
    let ignore_content = match std::fs::read_to_string(&stignore_path) {
        Ok(content) => content,
        Err(_) => return false, // No .stignore file means nothing is ignored
    };

    // Check if the path is in the ignore list
    ignore_content
        .lines()
        .any(|line| line.trim() == normalized_path)
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

    // Ensure the path starts with '/' for consistency
    let normalized_path = if folder_path.starts_with('/') {
        folder_path.to_string()
    } else {
        format!("/{}", folder_path)
    };

    // Read existing .stignore or create new content
    let mut ignore_content = std::fs::read_to_string(&stignore_path).unwrap_or_default();

    // Check if the path is already ignored
    if ignore_content
        .lines()
        .any(|line| line.trim() == normalized_path)
    {
        return StignoreResult::AlreadyIgnored {
            ignored_path: normalized_path,
        };
    }

    // Add the path to ignore content
    if !ignore_content.is_empty() && !ignore_content.ends_with('\n') {
        ignore_content.push('\n');
    }
    ignore_content.push_str(&normalized_path);
    ignore_content.push('\n');

    // Write back to .stignore
    match std::fs::write(&stignore_path, ignore_content) {
        Ok(_) => StignoreResult::Success {
            ignored_path: normalized_path.clone(),
            message: format!(
                "Successfully added '{}' to .stignore in category '{}'",
                normalized_path, category_name
            ),
        },
        Err(err) => StignoreResult::Error {
            message: format!("Failed to write .stignore file: {}", err),
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
