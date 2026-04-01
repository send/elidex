//! Bookmark persistence sub-store (design doc §22.4.2 BookmarkPersistence).

use std::time::SystemTime;

use crate::error::StorageError;

/// A bookmark entry (file or folder).
#[derive(Debug, Clone)]
pub struct BookmarkNode {
    pub id: i64,
    pub parent_id: Option<i64>,
    pub title: String,
    pub url: Option<String>,
    pub position: i64,
    pub date_added: i64,
    pub is_folder: bool,
    pub children: Vec<BookmarkNode>,
}

/// Data for creating a new bookmark.
#[derive(Debug, Clone)]
pub struct NewBookmark {
    pub title: String,
    pub url: Option<String>,
    pub is_folder: bool,
}

/// Fields to update on an existing bookmark.
#[derive(Debug, Clone, Default)]
pub struct BookmarkUpdate {
    pub title: Option<String>,
    pub url: Option<String>,
}

/// Recursively attach children to a bookmark node from the flat map.
fn attach_children_recursive(
    node: &mut BookmarkNode,
    map: &mut std::collections::HashMap<Option<i64>, Vec<BookmarkNode>>,
) {
    if let Some(mut kids) = map.remove(&Some(node.id)) {
        for kid in &mut kids {
            attach_children_recursive(kid, map);
        }
        node.children = kids;
    }
}

/// Zero-cost borrow wrapper around the browser.sqlite connection.
pub struct BookmarkStore<'db> {
    conn: &'db rusqlite::Connection,
}

impl<'db> BookmarkStore<'db> {
    pub(crate) fn new(conn: &'db rusqlite::Connection) -> Self {
        Self { conn }
    }

    /// Load the entire bookmark tree rooted at `parent_id` (or all roots if `None`).
    pub fn load_tree(&self) -> Result<Vec<BookmarkNode>, StorageError> {
        // Load all nodes in one query, then build tree in memory.
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, parent_id, title, url, position, date_added, is_folder \
                 FROM bookmarks ORDER BY position",
            )
            .map_err(StorageError::from)?;

        let flat: Vec<BookmarkNode> = stmt
            .query_map([], |row| {
                Ok(BookmarkNode {
                    id: row.get(0)?,
                    parent_id: row.get(1)?,
                    title: row.get(2)?,
                    url: row.get(3)?,
                    position: row.get(4)?,
                    date_added: row.get(5)?,
                    is_folder: row.get::<_, i32>(6)? != 0,
                    children: Vec::new(),
                })
            })
            .map_err(StorageError::from)?
            .filter_map(Result::ok)
            .collect();

        // Build tree: collect children per parent_id.
        let mut children_map: std::collections::HashMap<Option<i64>, Vec<BookmarkNode>> =
            std::collections::HashMap::new();
        for node in flat {
            children_map.entry(node.parent_id).or_default().push(node);
        }

        let mut roots = children_map.remove(&None).unwrap_or_default();
        for root in &mut roots {
            attach_children_recursive(root, &mut children_map);
        }
        Ok(roots)
    }

    /// Add a new bookmark under `parent_id`. Returns the new bookmark's ID.
    pub fn add(&self, parent_id: i64, bookmark: &NewBookmark) -> Result<i64, StorageError> {
        let now = super::system_time_to_unix(SystemTime::now());

        // Get next position for this parent.
        let max_pos: i64 = self
            .conn
            .query_row(
                "SELECT COALESCE(MAX(position), -1) FROM bookmarks WHERE parent_id = ?1",
                [parent_id],
                |row| row.get(0),
            )
            .map_err(StorageError::from)?;

        self.conn
            .execute(
                "INSERT INTO bookmarks (parent_id, title, url, position, date_added, is_folder) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![
                    parent_id,
                    bookmark.title,
                    bookmark.url,
                    max_pos + 1,
                    now,
                    i32::from(bookmark.is_folder),
                ],
            )
            .map_err(StorageError::from)?;

        Ok(self.conn.last_insert_rowid())
    }

    /// Update an existing bookmark's title and/or URL.
    pub fn update(&self, id: i64, changes: &BookmarkUpdate) -> Result<(), StorageError> {
        if let Some(ref title) = changes.title {
            self.conn
                .execute(
                    "UPDATE bookmarks SET title = ?1 WHERE id = ?2",
                    rusqlite::params![title, id],
                )
                .map_err(StorageError::from)?;
        }
        if let Some(ref url) = changes.url {
            self.conn
                .execute(
                    "UPDATE bookmarks SET url = ?1 WHERE id = ?2",
                    rusqlite::params![url, id],
                )
                .map_err(StorageError::from)?;
        }
        Ok(())
    }

    /// Remove a bookmark (and all descendants via CASCADE).
    pub fn remove(&self, id: i64) -> Result<(), StorageError> {
        self.conn
            .execute("DELETE FROM bookmarks WHERE id = ?1", [id])
            .map_err(StorageError::from)?;
        Ok(())
    }

    /// Move a bookmark to a new parent at a given position.
    pub fn move_to(&self, id: i64, new_parent: i64, position: usize) -> Result<(), StorageError> {
        // Shift existing items at or after the target position.
        self.conn
            .execute(
                "UPDATE bookmarks SET position = position + 1 \
                 WHERE parent_id = ?1 AND position >= ?2 AND id != ?3",
                rusqlite::params![new_parent, position as i64, id],
            )
            .map_err(StorageError::from)?;

        self.conn
            .execute(
                "UPDATE bookmarks SET parent_id = ?1, position = ?2 WHERE id = ?3",
                rusqlite::params![new_parent, position as i64, id],
            )
            .map_err(StorageError::from)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::browser_db::BrowserDb;

    fn test_db() -> (tempfile::TempDir, BrowserDb) {
        let dir = tempfile::tempdir().unwrap();
        let db = BrowserDb::open(dir.path()).unwrap();
        (dir, db)
    }

    /// Create a root folder to serve as the bookmark root.
    fn create_root(db: &BrowserDb) -> i64 {
        db.raw_connection()
            .execute(
                "INSERT INTO bookmarks (parent_id, title, is_folder, date_added, position) \
                 VALUES (NULL, 'Root', 1, 0, 0)",
                [],
            )
            .unwrap();
        db.raw_connection().last_insert_rowid()
    }

    #[test]
    fn add_and_load() {
        let (_dir, db) = test_db();
        let store = db.bookmarks();
        let root = create_root(&db);

        store
            .add(
                root,
                &NewBookmark {
                    title: "Example".into(),
                    url: Some("https://example.com".into()),
                    is_folder: false,
                },
            )
            .unwrap();

        let tree = store.load_tree().unwrap();
        assert_eq!(tree.len(), 1); // one root
        assert_eq!(tree[0].children.len(), 1);
        assert_eq!(tree[0].children[0].title, "Example");
    }

    #[test]
    fn update_title() {
        let (_dir, db) = test_db();
        let store = db.bookmarks();
        let root = create_root(&db);

        let id = store
            .add(
                root,
                &NewBookmark {
                    title: "Old".into(),
                    url: None,
                    is_folder: true,
                },
            )
            .unwrap();

        store
            .update(
                id,
                &BookmarkUpdate {
                    title: Some("New".into()),
                    ..Default::default()
                },
            )
            .unwrap();

        let tree = store.load_tree().unwrap();
        assert_eq!(tree[0].children[0].title, "New");
    }

    #[test]
    fn remove_cascades() {
        let (_dir, db) = test_db();
        let store = db.bookmarks();
        let root = create_root(&db);

        let folder = store
            .add(
                root,
                &NewBookmark {
                    title: "Folder".into(),
                    url: None,
                    is_folder: true,
                },
            )
            .unwrap();

        store
            .add(
                folder,
                &NewBookmark {
                    title: "Child".into(),
                    url: Some("https://child.com".into()),
                    is_folder: false,
                },
            )
            .unwrap();

        store.remove(folder).unwrap();

        let tree = store.load_tree().unwrap();
        assert!(tree[0].children.is_empty());
    }

    #[test]
    fn move_to_new_parent() {
        let (_dir, db) = test_db();
        let store = db.bookmarks();
        let root = create_root(&db);

        let f1 = store
            .add(
                root,
                &NewBookmark {
                    title: "Folder1".into(),
                    url: None,
                    is_folder: true,
                },
            )
            .unwrap();

        let f2 = store
            .add(
                root,
                &NewBookmark {
                    title: "Folder2".into(),
                    url: None,
                    is_folder: true,
                },
            )
            .unwrap();

        let bm = store
            .add(
                f1,
                &NewBookmark {
                    title: "Item".into(),
                    url: Some("https://item.com".into()),
                    is_folder: false,
                },
            )
            .unwrap();

        store.move_to(bm, f2, 0).unwrap();

        let tree = store.load_tree().unwrap();
        let f1_node = tree[0]
            .children
            .iter()
            .find(|n| n.title == "Folder1")
            .unwrap();
        let f2_node = tree[0]
            .children
            .iter()
            .find(|n| n.title == "Folder2")
            .unwrap();
        assert!(f1_node.children.is_empty());
        assert_eq!(f2_node.children.len(), 1);
        assert_eq!(f2_node.children[0].title, "Item");
    }
}
