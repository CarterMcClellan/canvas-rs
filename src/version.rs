use crate::scene::Shape;

/// Represents a single saved version/snapshot of the canvas state
#[derive(Clone, Debug, PartialEq)]
pub struct Version {
    /// Unique version ID (monotonically increasing)
    pub id: u64,
    /// Human-readable label (e.g., "Version 1")
    pub label: String,
    /// Timestamp when this version was created (milliseconds since epoch)
    pub created_at: f64,
    /// Snapshot of all shapes at this version
    pub shapes: Vec<Shape>,
}

impl Version {
    pub fn new(id: u64, label: String, created_at: f64, shapes: Vec<Shape>) -> Self {
        Self {
            id,
            label,
            created_at,
            shapes,
        }
    }
}

/// Version history manager
#[derive(Clone, Debug, PartialEq)]
pub struct VersionHistory {
    /// All saved versions, ordered by creation time
    pub versions: Vec<Version>,
    /// ID counter for generating unique version IDs
    pub next_id: u64,
    /// Currently active version index (None if working on unsaved changes)
    pub current_version_idx: Option<usize>,
}

impl Default for VersionHistory {
    fn default() -> Self {
        Self::new()
    }
}

impl VersionHistory {
    pub fn new() -> Self {
        Self {
            versions: Vec::new(),
            next_id: 1,
            current_version_idx: None,
        }
    }

    /// Save current state as a new version
    pub fn save_version(&mut self, shapes: Vec<Shape>, label: Option<String>, timestamp: f64) -> &Version {
        let version = Version::new(
            self.next_id,
            label.unwrap_or_else(|| format!("Version {}", self.next_id)),
            timestamp,
            shapes,
        );
        self.next_id += 1;
        self.versions.push(version);
        self.current_version_idx = Some(self.versions.len() - 1);
        self.versions.last().unwrap()
    }

    /// Get a specific version by index
    pub fn get_version(&self, idx: usize) -> Option<&Version> {
        self.versions.get(idx)
    }

    /// Get the number of saved versions
    pub fn len(&self) -> usize {
        self.versions.len()
    }

    /// Check if there are no saved versions
    pub fn is_empty(&self) -> bool {
        self.versions.is_empty()
    }

    /// Set the current version index (for restoring a version)
    pub fn set_current_version(&mut self, idx: usize) {
        if idx < self.versions.len() {
            self.current_version_idx = Some(idx);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{Shape, ShapeGeometry, ShapeStyle};

    fn create_test_shape() -> Shape {
        Shape::new(
            ShapeGeometry::rectangle(100.0, 50.0),
            ShapeStyle::default(),
        )
    }

    #[test]
    fn test_new_history() {
        let history = VersionHistory::new();
        assert!(history.is_empty());
        assert_eq!(history.next_id, 1);
        assert!(history.current_version_idx.is_none());
    }

    #[test]
    fn test_save_version() {
        let mut history = VersionHistory::new();
        let shapes = vec![create_test_shape()];

        history.save_version(shapes.clone(), None, 1000.0);

        assert_eq!(history.len(), 1);
        assert_eq!(history.next_id, 2);
        assert_eq!(history.current_version_idx, Some(0));

        let version = history.get_version(0).unwrap();
        assert_eq!(version.id, 1);
        assert_eq!(version.label, "Version 1");
        assert_eq!(version.shapes.len(), 1);
    }

    #[test]
    fn test_set_current_version() {
        let mut history = VersionHistory::new();
        let shapes = vec![create_test_shape()];

        history.save_version(shapes.clone(), None, 1000.0);
        history.save_version(shapes.clone(), None, 2000.0);

        assert_eq!(history.current_version_idx, Some(1));

        // Go back to first version
        history.set_current_version(0);
        assert_eq!(history.current_version_idx, Some(0));

        // Invalid index should not change current version
        history.set_current_version(99);
        assert_eq!(history.current_version_idx, Some(0));
    }
}
