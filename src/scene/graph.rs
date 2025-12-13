use super::shape::Shape;
use super::types::{BBox, ShapeStyle, Transform2D, Vec2};
use super::ShapeGeometry;
use std::collections::HashSet;

/// Scene graph for managing shapes
/// Provides efficient shape management with dirty tracking for rendering
pub struct SceneGraph {
    /// All shapes in the scene
    shapes: Vec<Shape>,
    /// IDs of shapes that need re-rendering
    dirty_shapes: HashSet<u64>,
    /// Whether the entire scene needs re-rendering
    scene_dirty: bool,
    /// Currently selected shape IDs
    selection: Vec<u64>,
}

impl Default for SceneGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl SceneGraph {
    /// Create a new empty scene graph
    pub fn new() -> Self {
        Self {
            shapes: Vec::new(),
            dirty_shapes: HashSet::new(),
            scene_dirty: true,
            selection: Vec::new(),
        }
    }

    /// Add a shape to the scene and return its ID
    pub fn add_shape(&mut self, shape: Shape) -> u64 {
        let id = shape.id;
        self.dirty_shapes.insert(id);
        self.scene_dirty = true;
        self.shapes.push(shape);
        id
    }

    /// Create and add a new shape with the given geometry and style
    pub fn create_shape(&mut self, geometry: ShapeGeometry, style: ShapeStyle) -> u64 {
        let shape = Shape::new(geometry, style);
        self.add_shape(shape)
    }

    /// Remove a shape by ID
    pub fn remove_shape(&mut self, id: u64) -> Option<Shape> {
        if let Some(pos) = self.shapes.iter().position(|s| s.id == id) {
            self.dirty_shapes.remove(&id);
            self.selection.retain(|&sid| sid != id);
            self.scene_dirty = true;
            Some(self.shapes.remove(pos))
        } else {
            None
        }
    }

    /// Get a shape by ID
    pub fn get_shape(&self, id: u64) -> Option<&Shape> {
        self.shapes.iter().find(|s| s.id == id)
    }

    /// Get a mutable reference to a shape by ID
    pub fn get_shape_mut(&mut self, id: u64) -> Option<&mut Shape> {
        let shape = self.shapes.iter_mut().find(|s| s.id == id);
        if let Some(s) = shape.as_ref() {
            self.dirty_shapes.insert(s.id);
            self.scene_dirty = true;
        }
        shape
    }

    /// Get all shapes
    pub fn shapes(&self) -> &[Shape] {
        &self.shapes
    }

    /// Get number of shapes
    pub fn len(&self) -> usize {
        self.shapes.len()
    }

    /// Check if scene is empty
    pub fn is_empty(&self) -> bool {
        self.shapes.is_empty()
    }

    /// Update a shape's transform
    pub fn set_transform(&mut self, id: u64, transform: Transform2D) {
        if let Some(shape) = self.shapes.iter_mut().find(|s| s.id == id) {
            shape.transform = transform;
            shape.dirty = true;
            self.dirty_shapes.insert(id);
            self.scene_dirty = true;
        }
    }

    /// Update a shape's style
    pub fn set_style(&mut self, id: u64, style: ShapeStyle) {
        if let Some(shape) = self.shapes.iter_mut().find(|s| s.id == id) {
            shape.style = style;
            shape.dirty = true;
            self.dirty_shapes.insert(id);
            self.scene_dirty = true;
        }
    }

    /// Update a shape's geometry
    pub fn set_geometry(&mut self, id: u64, geometry: ShapeGeometry) {
        if let Some(shape) = self.shapes.iter_mut().find(|s| s.id == id) {
            shape.geometry = geometry;
            shape.dirty = true;
            self.dirty_shapes.insert(id);
            self.scene_dirty = true;
        }
    }

    /// Check if the scene needs re-rendering
    pub fn is_dirty(&self) -> bool {
        self.scene_dirty
    }

    /// Get IDs of dirty shapes
    pub fn dirty_shape_ids(&self) -> &HashSet<u64> {
        &self.dirty_shapes
    }

    /// Clear dirty flags after rendering
    pub fn clear_dirty(&mut self) {
        self.dirty_shapes.clear();
        self.scene_dirty = false;
        for shape in &mut self.shapes {
            shape.dirty = false;
        }
    }

    /// Mark entire scene as dirty (force full re-render)
    pub fn mark_dirty(&mut self) {
        self.scene_dirty = true;
        for shape in &mut self.shapes {
            shape.dirty = true;
            self.dirty_shapes.insert(shape.id);
        }
    }

    // === Selection Management ===

    /// Get currently selected shape IDs
    pub fn selection(&self) -> &[u64] {
        &self.selection
    }

    /// Select a shape by ID
    pub fn select(&mut self, id: u64) {
        if self.get_shape(id).is_some() && !self.selection.contains(&id) {
            self.selection.push(id);
        }
    }

    /// Select multiple shapes
    pub fn select_multiple(&mut self, ids: &[u64]) {
        for &id in ids {
            self.select(id);
        }
    }

    /// Deselect a shape
    pub fn deselect(&mut self, id: u64) {
        self.selection.retain(|&sid| sid != id);
    }

    /// Clear selection
    pub fn clear_selection(&mut self) {
        self.selection.clear();
    }

    /// Check if a shape is selected
    pub fn is_selected(&self, id: u64) -> bool {
        self.selection.contains(&id)
    }

    /// Get selected shapes
    pub fn selected_shapes(&self) -> Vec<&Shape> {
        self.selection
            .iter()
            .filter_map(|&id| self.get_shape(id))
            .collect()
    }

    /// Get bounding box of selected shapes
    pub fn selection_bounds(&self) -> Option<BBox> {
        let selected = self.selected_shapes();
        if selected.is_empty() {
            return None;
        }

        let mut bounds = selected[0].world_bounds();
        for shape in &selected[1..] {
            bounds = bounds.union(&shape.world_bounds());
        }
        Some(bounds)
    }

    // === Hit Testing ===

    /// Find shape at point (returns topmost shape)
    pub fn hit_test(&self, point: Vec2) -> Option<u64> {
        // Iterate in reverse to get topmost shape first
        for shape in self.shapes.iter().rev() {
            if shape.contains_point(point) {
                return Some(shape.id);
            }
        }
        None
    }

    /// Find all shapes intersecting a rectangle
    pub fn query_rect(&self, rect: &BBox) -> Vec<u64> {
        self.shapes
            .iter()
            .filter(|shape| shape.world_bounds().intersects(rect))
            .map(|shape| shape.id)
            .collect()
    }

    // === Z-Order Management ===

    /// Move shape to front (top of z-order)
    pub fn bring_to_front(&mut self, id: u64) {
        if let Some(pos) = self.shapes.iter().position(|s| s.id == id) {
            let shape = self.shapes.remove(pos);
            self.shapes.push(shape);
            self.scene_dirty = true;
        }
    }

    /// Move shape to back (bottom of z-order)
    pub fn send_to_back(&mut self, id: u64) {
        if let Some(pos) = self.shapes.iter().position(|s| s.id == id) {
            let shape = self.shapes.remove(pos);
            self.shapes.insert(0, shape);
            self.scene_dirty = true;
        }
    }

    /// Move shape forward one position
    pub fn bring_forward(&mut self, id: u64) {
        if let Some(pos) = self.shapes.iter().position(|s| s.id == id) {
            if pos < self.shapes.len() - 1 {
                self.shapes.swap(pos, pos + 1);
                self.scene_dirty = true;
            }
        }
    }

    /// Move shape backward one position
    pub fn send_backward(&mut self, id: u64) {
        if let Some(pos) = self.shapes.iter().position(|s| s.id == id) {
            if pos > 0 {
                self.shapes.swap(pos, pos - 1);
                self.scene_dirty = true;
            }
        }
    }

    // === Bulk Operations ===

    /// Transform all selected shapes
    pub fn transform_selection(&mut self, delta_position: Vec2, delta_scale: Vec2) {
        for &id in &self.selection.clone() {
            if let Some(shape) = self.shapes.iter_mut().find(|s| s.id == id) {
                shape.transform.position += delta_position;
                shape.transform.scale *= delta_scale;
                shape.dirty = true;
                self.dirty_shapes.insert(id);
            }
        }
        if !self.selection.is_empty() {
            self.scene_dirty = true;
        }
    }

    /// Delete all selected shapes
    pub fn delete_selection(&mut self) {
        let to_delete: Vec<u64> = self.selection.clone();
        for id in to_delete {
            self.remove_shape(id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::Color;

    fn create_test_shape() -> Shape {
        Shape::new(
            ShapeGeometry::polygon(vec![
                Vec2::new(0.0, 0.0),
                Vec2::new(100.0, 0.0),
                Vec2::new(50.0, 100.0),
            ]),
            ShapeStyle::fill_only(Color::rgb(1.0, 0.0, 0.0)),
        )
    }

    #[test]
    fn test_add_and_get_shape() {
        let mut scene = SceneGraph::new();
        let shape = create_test_shape();
        let id = shape.id;
        scene.add_shape(shape);

        assert_eq!(scene.len(), 1);
        assert!(scene.get_shape(id).is_some());
    }

    #[test]
    fn test_remove_shape() {
        let mut scene = SceneGraph::new();
        let shape = create_test_shape();
        let id = shape.id;
        scene.add_shape(shape);

        let removed = scene.remove_shape(id);
        assert!(removed.is_some());
        assert_eq!(scene.len(), 0);
    }

    #[test]
    fn test_selection() {
        let mut scene = SceneGraph::new();
        let shape1 = create_test_shape();
        let shape2 = create_test_shape();
        let id1 = shape1.id;
        let id2 = shape2.id;
        scene.add_shape(shape1);
        scene.add_shape(shape2);

        scene.select(id1);
        assert!(scene.is_selected(id1));
        assert!(!scene.is_selected(id2));

        scene.select(id2);
        assert_eq!(scene.selection().len(), 2);

        scene.deselect(id1);
        assert!(!scene.is_selected(id1));
        assert!(scene.is_selected(id2));
    }

    #[test]
    fn test_dirty_tracking() {
        let mut scene = SceneGraph::new();
        assert!(scene.is_dirty()); // New scene is dirty

        scene.clear_dirty();
        assert!(!scene.is_dirty());

        let shape = create_test_shape();
        let id = shape.id;
        scene.add_shape(shape);
        assert!(scene.is_dirty());
        assert!(scene.dirty_shape_ids().contains(&id));
    }

    #[test]
    fn test_z_order() {
        let mut scene = SceneGraph::new();
        let shape1 = create_test_shape();
        let shape2 = create_test_shape();
        let id1 = shape1.id;
        let id2 = shape2.id;
        scene.add_shape(shape1);
        scene.add_shape(shape2);

        // shape2 should be on top initially
        assert_eq!(scene.shapes()[1].id, id2);

        scene.send_to_back(id2);
        assert_eq!(scene.shapes()[0].id, id2);
        assert_eq!(scene.shapes()[1].id, id1);

        scene.bring_to_front(id2);
        assert_eq!(scene.shapes()[1].id, id2);
    }
}
