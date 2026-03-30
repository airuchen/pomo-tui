use std::collections::HashMap;
use uuid::Uuid;

use crate::db::todos::TodoRow;

#[derive(Debug, Clone)]
pub struct TodoItem {
    pub id: Uuid,
    pub parent_id: Option<Uuid>,
    pub title: String,
    pub done: bool,
    pub priority: String,
    pub sort_order: i64,
    pub children: Vec<Uuid>,
    pub expanded: bool,
    pub session_count: i64,
}

#[derive(Debug, Default)]
pub struct TodoTree {
    pub items: HashMap<Uuid, TodoItem>,
    pub roots: Vec<Uuid>,
}

impl TodoTree {
    pub fn from_rows(rows: Vec<TodoRow>) -> Self {
        let mut items = HashMap::new();
        let mut roots = Vec::new();

        // First pass: create all items
        for row in &rows {
            let id = Uuid::parse_str(&row.id).expect("invalid UUID in todos table");
            let parent_id = row
                .parent_id
                .as_deref()
                .map(|p| Uuid::parse_str(p).expect("invalid parent UUID in todos table"));
            items.insert(
                id,
                TodoItem {
                    id,
                    parent_id,
                    title: row.title.clone(),
                    done: row.done != 0,
                    priority: row.priority.clone(),
                    sort_order: row.sort_order,
                    children: Vec::new(),
                    expanded: false,
                    session_count: 0,
                },
            );
        }

        // Second pass: build parent-child relationships
        let ids: Vec<(Uuid, Option<Uuid>)> = items
            .values()
            .map(|item| (item.id, item.parent_id))
            .collect();

        for (id, parent_id) in ids {
            match parent_id {
                Some(pid) => {
                    if let Some(parent) = items.get_mut(&pid) {
                        parent.children.push(id);
                    }
                }
                None => roots.push(id),
            }
        }

        // Sort children by (priority ASC, sort_order ASC)
        let sort_keys: HashMap<Uuid, (String, i64)> = items
            .iter()
            .map(|(id, item)| (*id, (item.priority.clone(), item.sort_order)))
            .collect();

        let sort_fn = |id: &Uuid| sort_keys.get(id).cloned().unwrap_or(("B".to_string(), 0));

        for item in items.values_mut() {
            item.children.sort_by(|a, b| sort_fn(a).cmp(&sort_fn(b)));
        }
        roots.sort_by(|a, b| sort_fn(a).cmp(&sort_fn(b)));

        Self { items, roots }
    }

    /// Returns visible items as (depth, &TodoItem) via depth-first traversal,
    /// respecting expanded flags.
    pub fn visible_items(&self) -> Vec<(usize, &TodoItem)> {
        let mut result = Vec::new();
        for &root_id in &self.roots {
            self.collect_visible(root_id, 0, &mut result);
        }
        result
    }

    fn collect_visible<'a>(
        &'a self,
        id: Uuid,
        depth: usize,
        result: &mut Vec<(usize, &'a TodoItem)>,
    ) {
        if let Some(item) = self.items.get(&id) {
            result.push((depth, item));
            if item.expanded {
                for &child_id in &item.children {
                    self.collect_visible(child_id, depth + 1, result);
                }
            }
        }
    }

    pub fn toggle_expanded(&mut self, id: Uuid) {
        if let Some(item) = self.items.get_mut(&id) {
            if !item.children.is_empty() {
                item.expanded = !item.expanded;
            }
        }
    }

    pub fn expand(&mut self, id: Uuid) {
        if let Some(item) = self.items.get_mut(&id) {
            if !item.children.is_empty() {
                item.expanded = true;
            }
        }
    }

    pub fn collapse(&mut self, id: Uuid) {
        if let Some(item) = self.items.get_mut(&id) {
            item.expanded = false;
        }
    }

    /// Get the parent_id of the item at the given cursor position.
    /// Used for adding siblings (new item shares the same parent).
    pub fn parent_of_visible(&self, cursor: usize) -> Option<Option<Uuid>> {
        let visible = self.visible_items();
        visible.get(cursor).map(|(_, item)| item.parent_id)
    }

    /// Get the id of the item at the given cursor position.
    pub fn id_at_cursor(&self, cursor: usize) -> Option<Uuid> {
        let visible = self.visible_items();
        visible.get(cursor).map(|(_, item)| item.id)
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_row(id: &str, parent_id: Option<&str>, title: &str, order: i64) -> TodoRow {
        make_row_with_priority(id, parent_id, title, order, "B")
    }

    fn make_row_with_priority(
        id: &str,
        parent_id: Option<&str>,
        title: &str,
        order: i64,
        priority: &str,
    ) -> TodoRow {
        TodoRow {
            id: id.to_string(),
            parent_id: parent_id.map(|s| s.to_string()),
            title: title.to_string(),
            done: 0,
            priority: priority.to_string(),
            sort_order: order,
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    #[test]
    fn test_empty_tree() {
        let tree = TodoTree::from_rows(vec![]);
        assert!(tree.is_empty());
        assert!(tree.visible_items().is_empty());
    }

    #[test]
    fn test_flat_list() {
        let id1 = Uuid::new_v4().to_string();
        let id2 = Uuid::new_v4().to_string();
        let rows = vec![
            make_row(&id1, None, "First", 0),
            make_row(&id2, None, "Second", 1),
        ];
        let tree = TodoTree::from_rows(rows);
        let visible = tree.visible_items();
        assert_eq!(visible.len(), 2);
        assert_eq!(visible[0].1.title, "First");
        assert_eq!(visible[1].1.title, "Second");
        assert_eq!(visible[0].0, 0); // depth 0
    }

    #[test]
    fn test_nested_collapsed_by_default() {
        let parent_id = Uuid::new_v4().to_string();
        let child_id = Uuid::new_v4().to_string();
        let rows = vec![
            make_row(&parent_id, None, "Parent", 0),
            make_row(&child_id, Some(&parent_id), "Child", 0),
        ];
        let tree = TodoTree::from_rows(rows);
        // Children hidden by default (collapsed)
        let visible = tree.visible_items();
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].1.title, "Parent");
    }

    #[test]
    fn test_expand_shows_children() {
        let parent_id = Uuid::new_v4();
        let child_id = Uuid::new_v4();
        let rows = vec![
            make_row(&parent_id.to_string(), None, "Parent", 0),
            make_row(
                &child_id.to_string(),
                Some(&parent_id.to_string()),
                "Child",
                0,
            ),
        ];
        let mut tree = TodoTree::from_rows(rows);
        tree.expand(parent_id);
        let visible = tree.visible_items();
        assert_eq!(visible.len(), 2);
        assert_eq!(visible[0].1.title, "Parent");
        assert_eq!(visible[0].0, 0);
        assert_eq!(visible[1].1.title, "Child");
        assert_eq!(visible[1].0, 1);
    }

    #[test]
    fn test_collapse_hides_children() {
        let parent_id = Uuid::new_v4();
        let child_id = Uuid::new_v4();
        let rows = vec![
            make_row(&parent_id.to_string(), None, "Parent", 0),
            make_row(
                &child_id.to_string(),
                Some(&parent_id.to_string()),
                "Child",
                0,
            ),
        ];
        let mut tree = TodoTree::from_rows(rows);
        tree.expand(parent_id);
        assert_eq!(tree.visible_items().len(), 2);
        tree.collapse(parent_id);
        assert_eq!(tree.visible_items().len(), 1);
    }

    #[test]
    fn test_sort_order_respected() {
        let id_a = Uuid::new_v4().to_string();
        let id_b = Uuid::new_v4().to_string();
        let id_c = Uuid::new_v4().to_string();
        let rows = vec![
            make_row(&id_c, None, "Third", 2),
            make_row(&id_a, None, "First", 0),
            make_row(&id_b, None, "Second", 1),
        ];
        let tree = TodoTree::from_rows(rows);
        let visible = tree.visible_items();
        assert_eq!(visible[0].1.title, "First");
        assert_eq!(visible[1].1.title, "Second");
        assert_eq!(visible[2].1.title, "Third");
    }

    #[test]
    fn test_priority_sorts_before_sort_order() {
        let id_a = Uuid::new_v4().to_string();
        let id_b = Uuid::new_v4().to_string();
        let id_c = Uuid::new_v4().to_string();
        let rows = vec![
            make_row_with_priority(&id_a, None, "Normal task", 0, "B"),
            make_row_with_priority(&id_b, None, "Low priority", 1, "C"),
            make_row_with_priority(&id_c, None, "High priority", 2, "A"),
        ];
        let tree = TodoTree::from_rows(rows);
        let visible = tree.visible_items();
        assert_eq!(visible[0].1.title, "High priority");
        assert_eq!(visible[0].1.priority, "A");
        assert_eq!(visible[1].1.title, "Normal task");
        assert_eq!(visible[1].1.priority, "B");
        assert_eq!(visible[2].1.title, "Low priority");
        assert_eq!(visible[2].1.priority, "C");
    }

    #[test]
    fn test_priority_sorts_children_independently() {
        let parent_id = Uuid::new_v4();
        let child_a = Uuid::new_v4();
        let child_b = Uuid::new_v4();
        let rows = vec![
            make_row(&parent_id.to_string(), None, "Parent", 0),
            make_row_with_priority(
                &child_b.to_string(),
                Some(&parent_id.to_string()),
                "Low child",
                0,
                "C",
            ),
            make_row_with_priority(
                &child_a.to_string(),
                Some(&parent_id.to_string()),
                "High child",
                1,
                "A",
            ),
        ];
        let mut tree = TodoTree::from_rows(rows);
        tree.expand(parent_id);
        let visible = tree.visible_items();
        assert_eq!(visible.len(), 3);
        assert_eq!(visible[1].1.title, "High child");
        assert_eq!(visible[2].1.title, "Low child");
    }
}
