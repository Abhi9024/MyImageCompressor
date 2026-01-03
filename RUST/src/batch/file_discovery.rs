//! File discovery for batch processing.

use std::path::{Path, PathBuf};

use crate::error::{MedImgError, Result};

/// File discovery for finding DICOM files.
pub struct FileDiscovery {
    /// Whether to scan recursively.
    recursive: bool,

    /// File patterns to match.
    patterns: Vec<String>,

    /// Maximum depth for recursive scanning (None = unlimited).
    max_depth: Option<usize>,

    /// Whether to follow symbolic links.
    follow_symlinks: bool,
}

impl Default for FileDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

impl FileDiscovery {
    /// Create a new file discovery with default settings.
    pub fn new() -> Self {
        Self {
            recursive: false,
            patterns: vec!["*.dcm".to_string(), "*.DCM".to_string()],
            max_depth: None,
            follow_symlinks: false,
        }
    }

    /// Enable recursive scanning.
    pub fn recursive(mut self, recursive: bool) -> Self {
        self.recursive = recursive;
        self
    }

    /// Set maximum depth for recursive scanning.
    pub fn max_depth(mut self, depth: usize) -> Self {
        self.max_depth = Some(depth);
        self
    }

    /// Set file patterns to match.
    pub fn patterns(mut self, patterns: Vec<String>) -> Self {
        self.patterns = patterns;
        self
    }

    /// Add a pattern to match.
    pub fn pattern(mut self, pattern: &str) -> Self {
        self.patterns.push(pattern.to_string());
        self
    }

    /// Enable following symbolic links.
    pub fn follow_symlinks(mut self, follow: bool) -> Self {
        self.follow_symlinks = follow;
        self
    }

    /// Discover files in the given directory.
    pub fn discover(&self, dir: &Path) -> Result<Vec<PathBuf>> {
        if !dir.exists() {
            return Err(MedImgError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Directory not found: {}", dir.display()),
            )));
        }

        if !dir.is_dir() {
            return Err(MedImgError::Validation(format!(
                "Not a directory: {}",
                dir.display()
            )));
        }

        let mut files = Vec::new();
        self.discover_recursive(dir, 0, &mut files)?;

        // Sort by path for deterministic ordering
        files.sort();

        Ok(files)
    }

    /// Recursive file discovery.
    fn discover_recursive(
        &self,
        dir: &Path,
        depth: usize,
        files: &mut Vec<PathBuf>,
    ) -> Result<()> {
        // Check depth limit
        if let Some(max) = self.max_depth {
            if depth > max {
                return Ok(());
            }
        }

        let entries = std::fs::read_dir(dir).map_err(|e| {
            MedImgError::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to read directory {}: {}", dir.display(), e),
            ))
        })?;

        for entry in entries {
            let entry = entry.map_err(MedImgError::Io)?;
            let path = entry.path();

            // Handle symlinks
            let metadata = if self.follow_symlinks {
                std::fs::metadata(&path)
            } else {
                std::fs::symlink_metadata(&path)
            };

            let metadata = match metadata {
                Ok(m) => m,
                Err(_) => continue, // Skip files we can't access
            };

            if metadata.is_dir() {
                if self.recursive {
                    self.discover_recursive(&path, depth + 1, files)?;
                }
            } else if metadata.is_file() {
                if self.matches_pattern(&path) {
                    files.push(path);
                }
            }
        }

        Ok(())
    }

    /// Check if a path matches any of the patterns.
    fn matches_pattern(&self, path: &Path) -> bool {
        let file_name = match path.file_name() {
            Some(name) => name.to_string_lossy().to_lowercase(),
            None => return false,
        };

        for pattern in &self.patterns {
            if self.glob_match(&file_name, &pattern.to_lowercase()) {
                return true;
            }
        }

        false
    }

    /// Simple glob matching (supports * and ?).
    fn glob_match(&self, text: &str, pattern: &str) -> bool {
        let text_chars: Vec<char> = text.chars().collect();
        let pattern_chars: Vec<char> = pattern.chars().collect();

        self.glob_match_recursive(&text_chars, 0, &pattern_chars, 0)
    }

    fn glob_match_recursive(
        &self,
        text: &[char],
        ti: usize,
        pattern: &[char],
        pi: usize,
    ) -> bool {
        // Both exhausted - match
        if ti == text.len() && pi == pattern.len() {
            return true;
        }

        // Pattern exhausted but text remains - no match
        if pi == pattern.len() {
            return false;
        }

        let pc = pattern[pi];

        if pc == '*' {
            // Try matching zero or more characters
            for i in ti..=text.len() {
                if self.glob_match_recursive(text, i, pattern, pi + 1) {
                    return true;
                }
            }
            false
        } else if ti < text.len() && (pc == '?' || pc == text[ti]) {
            // Single character match
            self.glob_match_recursive(text, ti + 1, pattern, pi + 1)
        } else {
            false
        }
    }
}

/// Discover files matching a pattern in a directory.
pub fn discover_files(dir: &Path, pattern: &str, recursive: bool) -> Result<Vec<PathBuf>> {
    FileDiscovery::new()
        .patterns(vec![pattern.to_string()])
        .recursive(recursive)
        .discover(dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_directory() -> TempDir {
        let dir = TempDir::new().unwrap();

        // Create test files
        fs::write(dir.path().join("test1.dcm"), "content").unwrap();
        fs::write(dir.path().join("test2.DCM"), "content").unwrap();
        fs::write(dir.path().join("test3.txt"), "content").unwrap();

        // Create subdirectory
        let subdir = dir.path().join("subdir");
        fs::create_dir(&subdir).unwrap();
        fs::write(subdir.join("nested.dcm"), "content").unwrap();

        dir
    }

    #[test]
    fn test_discovery_non_recursive() {
        let dir = create_test_directory();

        let discovery = FileDiscovery::new().recursive(false);
        let files = discovery.discover(dir.path()).unwrap();

        assert_eq!(files.len(), 2); // test1.dcm and test2.DCM
    }

    #[test]
    fn test_discovery_recursive() {
        let dir = create_test_directory();

        let discovery = FileDiscovery::new().recursive(true);
        let files = discovery.discover(dir.path()).unwrap();

        assert_eq!(files.len(), 3); // test1.dcm, test2.DCM, nested.dcm
    }

    #[test]
    fn test_discovery_custom_pattern() {
        let dir = create_test_directory();

        let discovery = FileDiscovery::new()
            .patterns(vec!["*.txt".to_string()])
            .recursive(false);
        let files = discovery.discover(dir.path()).unwrap();

        assert_eq!(files.len(), 1);
        assert!(files[0].file_name().unwrap().to_string_lossy().contains("test3"));
    }

    #[test]
    fn test_discovery_max_depth() {
        let dir = create_test_directory();

        let discovery = FileDiscovery::new()
            .recursive(true)
            .max_depth(0); // Only top level
        let files = discovery.discover(dir.path()).unwrap();

        assert_eq!(files.len(), 2); // test1.dcm and test2.DCM
    }

    #[test]
    fn test_discovery_nonexistent_directory() {
        let discovery = FileDiscovery::new();
        let result = discovery.discover(Path::new("/nonexistent/path"));
        assert!(result.is_err());
    }

    #[test]
    fn test_glob_match() {
        let discovery = FileDiscovery::new();

        assert!(discovery.glob_match("test.dcm", "*.dcm"));
        assert!(discovery.glob_match("test.dcm", "test.dcm"));
        assert!(discovery.glob_match("test.dcm", "test.*"));
        assert!(discovery.glob_match("test.dcm", "*.*"));
        assert!(discovery.glob_match("test.dcm", "t?st.dcm"));
        assert!(!discovery.glob_match("test.dcm", "*.txt"));
        assert!(!discovery.glob_match("test.dcm", "foo.dcm"));
    }

    #[test]
    fn test_discover_files_function() {
        let dir = create_test_directory();

        let files = discover_files(dir.path(), "*.dcm", true).unwrap();
        assert_eq!(files.len(), 3);
    }
}
