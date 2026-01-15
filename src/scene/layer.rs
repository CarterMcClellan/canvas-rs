use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};

/// Global group ID counter
static NEXT_GROUP_ID: AtomicU64 = AtomicU64::new(1);

fn generate_group_id() -> u64 {
    NEXT_GROUP_ID.fetch_add(1, Ordering::Relaxed)
}

/// Counters for auto-generating group names
static NEXT_GROUP_NUM: AtomicU64 = AtomicU64::new(1);

fn generate_group_name() -> String {
    let num = NEXT_GROUP_NUM.fetch_add(1, Ordering::Relaxed);
    format!("Group {}", num)
}

/// A node in the layer hierarchy - either a shape reference or a group
#[derive(Clone, Debug, PartialEq)]
pub enum LayerNode {
    /// Reference to a shape by its ID
    Shape { shape_id: u64 },
    /// A group containing other nodes
    Group {
        id: u64,
        name: String,
        children: Vec<LayerNode>,
        expanded: bool,
    },
}

impl LayerNode {
    /// Create a new shape node
    pub fn shape(shape_id: u64) -> Self {
        LayerNode::Shape { shape_id }
    }

    /// Create a new group node
    pub fn group(name: String) -> Self {
        LayerNode::Group {
            id: generate_group_id(),
            name,
            children: Vec::new(),
            expanded: true,
        }
    }

    /// Create a new group with auto-generated name
    pub fn new_group() -> Self {
        Self::group(generate_group_name())
    }

    /// Get the ID of this node (shape_id for shapes, group id for groups)
    pub fn id(&self) -> u64 {
        match self {
            LayerNode::Shape { shape_id } => *shape_id,
            LayerNode::Group { id, .. } => *id,
        }
    }

    /// Check if this node is a shape
    pub fn is_shape(&self) -> bool {
        matches!(self, LayerNode::Shape { .. })
    }

    /// Check if this node is a group
    pub fn is_group(&self) -> bool {
        matches!(self, LayerNode::Group { .. })
    }

    /// Get all shape IDs contained in this node (recursively for groups)
    pub fn all_shape_ids(&self) -> Vec<u64> {
        match self {
            LayerNode::Shape { shape_id } => vec![*shape_id],
            LayerNode::Group { children, .. } => {
                children.iter().flat_map(|c| c.all_shape_ids()).collect()
            }
        }
    }

    /// Check if this node contains a specific shape ID (recursively)
    pub fn contains_shape(&self, target_id: u64) -> bool {
        match self {
            LayerNode::Shape { shape_id } => *shape_id == target_id,
            LayerNode::Group { children, .. } => {
                children.iter().any(|c| c.contains_shape(target_id))
            }
        }
    }
}

/// Manages the hierarchical layer structure
#[derive(Clone, Debug, PartialEq)]
pub struct LayerTree {
    /// Top-level nodes in the layer hierarchy
    pub nodes: Vec<LayerNode>,
}

impl Default for LayerTree {
    fn default() -> Self {
        Self::new()
    }
}

impl LayerTree {
    /// Create a new empty layer tree
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    /// Create a layer tree from a list of shape IDs
    pub fn from_shapes(shape_ids: &[u64]) -> Self {
        Self {
            nodes: shape_ids.iter().map(|&id| LayerNode::shape(id)).collect(),
        }
    }

    /// Add a shape to the top level
    pub fn add_shape(&mut self, shape_id: u64) {
        self.nodes.push(LayerNode::shape(shape_id));
    }

    /// Remove a shape from anywhere in the tree
    pub fn remove_shape(&mut self, shape_id: u64) {
        Self::remove_shape_recursive(&mut self.nodes, shape_id);
    }

    fn remove_shape_recursive(nodes: &mut Vec<LayerNode>, shape_id: u64) {
        nodes.retain(|node| {
            match node {
                LayerNode::Shape { shape_id: id } => *id != shape_id,
                LayerNode::Group { .. } => true, // Keep groups, we'll recurse into them
            }
        });

        for node in nodes.iter_mut() {
            if let LayerNode::Group { children, .. } = node {
                Self::remove_shape_recursive(children, shape_id);
            }
        }
    }

    /// Get all shape IDs in the tree in order
    pub fn all_shape_ids(&self) -> Vec<u64> {
        self.nodes.iter().flat_map(|n| n.all_shape_ids()).collect()
    }

    /// Create a group from selected shape IDs
    /// Returns the group ID if successful
    pub fn group_shapes(&mut self, shape_ids: &[u64]) -> Option<u64> {
        if shape_ids.len() < 2 {
            return None;
        }

        let shape_set: HashSet<_> = shape_ids.iter().copied().collect();

        // Find nodes to group and their first position
        let mut first_idx: Option<usize> = None;
        let mut nodes_to_group: Vec<LayerNode> = Vec::new();

        // Collect nodes that match the shape IDs (at top level for now)
        let mut i = 0;
        while i < self.nodes.len() {
            let should_include = match &self.nodes[i] {
                LayerNode::Shape { shape_id } => shape_set.contains(shape_id),
                LayerNode::Group { .. } => {
                    // Check if all shapes in this group are in the selection
                    let group_shapes: HashSet<_> = self.nodes[i].all_shape_ids().into_iter().collect();
                    !group_shapes.is_empty() && group_shapes.is_subset(&shape_set)
                }
            };

            if should_include {
                if first_idx.is_none() {
                    first_idx = Some(i);
                }
                nodes_to_group.push(self.nodes.remove(i));
            } else {
                i += 1;
            }
        }

        if nodes_to_group.len() < 2 {
            // Put nodes back if we couldn't form a group
            for node in nodes_to_group.into_iter().rev() {
                if let Some(idx) = first_idx {
                    self.nodes.insert(idx, node);
                }
            }
            return None;
        }

        // Create the group
        let group = LayerNode::Group {
            id: generate_group_id(),
            name: generate_group_name(),
            children: nodes_to_group,
            expanded: true,
        };
        let group_id = group.id();

        // Insert at the first position
        let insert_idx = first_idx.unwrap_or(self.nodes.len());
        self.nodes.insert(insert_idx, group);

        Some(group_id)
    }

    /// Ungroup a group by ID, moving its children to the group's position
    pub fn ungroup(&mut self, group_id: u64) -> bool {
        Self::ungroup_recursive(&mut self.nodes, group_id)
    }

    fn ungroup_recursive(nodes: &mut Vec<LayerNode>, group_id: u64) -> bool {
        for i in 0..nodes.len() {
            if let LayerNode::Group { id, .. } = &nodes[i] {
                if *id == group_id {
                    // Remove the group and insert its children at the same position
                    let children = if let LayerNode::Group { children, .. } = nodes.remove(i) {
                        children
                    } else {
                        unreachable!()
                    };
                    for (j, child) in children.into_iter().enumerate() {
                        nodes.insert(i + j, child);
                    }
                    return true;
                }
            }

            // Recurse into child groups
            if let LayerNode::Group { children, .. } = &mut nodes[i] {
                if Self::ungroup_recursive(children, group_id) {
                    return true;
                }
            }
        }
        false
    }

    /// Toggle the expanded state of a group
    pub fn toggle_expanded(&mut self, group_id: u64) {
        Self::toggle_expanded_recursive(&mut self.nodes, group_id);
    }

    fn toggle_expanded_recursive(nodes: &mut [LayerNode], group_id: u64) {
        for node in nodes.iter_mut() {
            if let LayerNode::Group { id, expanded, children, .. } = node {
                if *id == group_id {
                    *expanded = !*expanded;
                    return;
                }
                Self::toggle_expanded_recursive(children, group_id);
            }
        }
    }

    /// Rename a group
    pub fn rename_group(&mut self, group_id: u64, new_name: String) {
        Self::rename_group_recursive(&mut self.nodes, group_id, new_name);
    }

    fn rename_group_recursive(nodes: &mut [LayerNode], group_id: u64, new_name: String) {
        for node in nodes.iter_mut() {
            if let LayerNode::Group { id, name, children, .. } = node {
                if *id == group_id {
                    *name = new_name;
                    return;
                }
                Self::rename_group_recursive(children, group_id, new_name.clone());
            }
        }
    }

    /// Find all shape IDs that are descendants of a group
    pub fn get_group_shape_ids(&self, group_id: u64) -> Vec<u64> {
        Self::find_group_shapes(&self.nodes, group_id)
    }

    fn find_group_shapes(nodes: &[LayerNode], group_id: u64) -> Vec<u64> {
        for node in nodes {
            if let LayerNode::Group { id, children, .. } = node {
                if *id == group_id {
                    return node.all_shape_ids();
                }
                let result = Self::find_group_shapes(children, group_id);
                if !result.is_empty() {
                    return result;
                }
            }
        }
        Vec::new()
    }

    /// Find all shape IDs that should be selected when clicking on a shape.
    /// If the shape is in a group, returns all shapes in the top-most group containing it.
    /// If not in a group, returns just the clicked shape ID.
    pub fn get_selection_for_shape(&self, shape_id: u64) -> Vec<u64> {
        Self::find_selection_for_shape(&self.nodes, shape_id)
            .unwrap_or_else(|| vec![shape_id])
    }

    fn find_selection_for_shape(nodes: &[LayerNode], shape_id: u64) -> Option<Vec<u64>> {
        for node in nodes {
            match node {
                LayerNode::Shape { shape_id: id } => {
                    if *id == shape_id {
                        // Found the shape at top level - not in a group
                        return Some(vec![shape_id]);
                    }
                }
                LayerNode::Group { .. } => {
                    // Check if this group contains the shape (directly or nested)
                    if node.contains_shape(shape_id) {
                        // This group contains our shape - return all shapes in this group
                        return Some(node.all_shape_ids());
                    }
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layer_tree_from_shapes() {
        let tree = LayerTree::from_shapes(&[1, 2, 3]);
        assert_eq!(tree.all_shape_ids(), vec![1, 2, 3]);
    }

    #[test]
    fn test_group_shapes() {
        let mut tree = LayerTree::from_shapes(&[1, 2, 3, 4]);
        let group_id = tree.group_shapes(&[2, 3]).unwrap();

        assert!(group_id > 0);
        assert_eq!(tree.nodes.len(), 3); // 1, group, 4
        assert_eq!(tree.all_shape_ids(), vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_ungroup() {
        let mut tree = LayerTree::from_shapes(&[1, 2, 3, 4]);
        let group_id = tree.group_shapes(&[2, 3]).unwrap();

        assert!(tree.ungroup(group_id));
        assert_eq!(tree.nodes.len(), 4);
        assert_eq!(tree.all_shape_ids(), vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_remove_shape() {
        let mut tree = LayerTree::from_shapes(&[1, 2, 3]);
        tree.remove_shape(2);
        assert_eq!(tree.all_shape_ids(), vec![1, 3]);
    }

    #[test]
    fn test_nested_groups() {
        let mut tree = LayerTree::from_shapes(&[1, 2, 3, 4, 5]);
        tree.group_shapes(&[2, 3]).unwrap();
        // Tree: 1, group(2,3), 4, 5

        let outer_group_id = tree.group_shapes(&[1, 2, 3]).unwrap(); // Groups shape 1 and the inner group
        assert!(outer_group_id > 0);

        // All shapes should still be accessible
        let all_ids = tree.all_shape_ids();
        assert_eq!(all_ids.len(), 5);
    }
}
