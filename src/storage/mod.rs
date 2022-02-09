use crate::weibo::post::Post;
use rusqlite::types::{FromSql, ToSql};
use rusqlite::{named_params, Connection};
use std::collections::HashSet;
use std::path::Path;

// Storage 实际上是一个 sqlite 数据库。它包含以下表:
// post, post_tombstone

pub struct Storage {
    conn: Connection,
}

pub struct PostStorage<'a> {
    storage: &'a Storage,
}

pub struct PostTombstoneStorage<'a> {
    storage: &'a Storage,
}

pub struct SettingsStorage<'a> {
    storage: &'a Storage,
}

impl Storage {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Storage, anyhow::Error> {
        let pragma = r#"
            PRAGMA synchronous = 0;
            PRAGMA cache_size = 1000000;
            PRAGMA temp_store = MEMORY;
        "#;

        let post_table_creation = r#"
            create table if not exists post (
                id integer primary key,
                url text not null,
                content text not null
            );
        "#;

        let post_tombstone_table_creation = r#"
            create table if not exists post_tombstone (
                id integer primary key,
                url text not null
            );
        "#;

        let settings_table_creation = r#"
            create table if not exists settings (
                name text unique,
                value blob not null
            );
        "#;

        let conn = Connection::open(path)?;
        conn.execute_batch(pragma)?;
        conn.execute(post_table_creation, [])?;
        conn.execute(post_tombstone_table_creation, [])?;
        conn.execute(settings_table_creation, [])?;

        Ok(Storage { conn })
    }

    pub fn posts(&self) -> PostStorage<'_> {
        PostStorage { storage: self }
    }

    pub fn post_tombstones(&self) -> PostTombstoneStorage<'_> {
        PostTombstoneStorage { storage: self }
    }

    pub fn settings(&self) -> SettingsStorage<'_> {
        SettingsStorage { storage: self }
    }
}

impl<'a> PostStorage<'a> {
    pub fn add(&self, post: &Post) -> Result<(), anyhow::Error> {
        let sql = "insert or replace into post (id, url, content) values (:id, :url, :content)";

        let content = serde_json::to_string_pretty(post)?;
        self.storage.conn.execute(
            sql,
            named_params! {
                ":id": post.id,
                ":url": post.url(),
                ":content": content,
            },
        )?;
        Ok(())
    }

    pub fn batch_add(&mut self, posts: &[Post]) -> Result<(), anyhow::Error> {
        let sql = "insert or replace into post (id, url, content) values (:id, :url, :content)";
        let tx = self.storage.conn.unchecked_transaction()?;
        {
            // seems NLL not working here.
            let mut stmt = tx.prepare_cached(sql)?;
            for post in posts {
                let content = serde_json::to_string_pretty(post)?;
                stmt.execute(named_params! {
                    ":id": post.id,
                    ":url": post.url(),
                    ":content": content,
                })?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn get_posts(&self, since_id: i64, limit: usize) -> Result<Vec<Post>, anyhow::Error> {
        let sql = "select content from post where id > :since_id order by id limit :limit";
        let mut stmt = self.storage.conn.prepare(sql)?;
        let mut rows = stmt.query(named_params! {
            ":since_id": since_id,
            ":limit": limit,
        })?;

        let mut posts = vec![];
        while let Some(row) = rows.next()? {
            let content: String = row.get(0)?;
            let post: Post = serde_json::from_str(&content)?;
            posts.push(post);
        }
        Ok(posts)
    }

    pub fn get_by_id(&self, post_id: i64) -> Result<Option<Post>, anyhow::Error> {
        let mut stmt = self
            .storage
            .conn
            .prepare("select content FROM post where id = :post_id")?;
        let mut rows = stmt.query(named_params! {":post_id": post_id})?;
        match rows.next() {
            Ok(Some(row)) => {
                let content: String = row.get(0)?;
                let post: Post = serde_json::from_str(&content)?;
                Ok(Some(post))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn get_by_url(&self, url: &str) -> Result<Option<Post>, anyhow::Error> {
        let mut stmt = self
            .storage
            .conn
            .prepare("select content FROM post where url = :url")?;
        let mut rows = stmt.query(named_params! {":url": url})?;
        match rows.next() {
            Ok(Some(row)) => {
                let content: String = row.get(0)?;
                let post: Post = serde_json::from_str(&content)?;
                Ok(Some(post))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn delete_all(&self) -> Result<(), anyhow::Error> {
        self.storage.conn.execute("delete from post", [])?;
        Ok(())
    }
}

impl<'a> PostTombstoneStorage<'a> {
    pub fn add(&self, post: &Post) -> Result<(), anyhow::Error> {
        let sql = "insert or replace into post_tombstone (id, url) values (:id, :url)";

        self.storage.conn.execute(
            sql,
            named_params! {
                ":id": post.id,
                ":url": post.url(),
            },
        )?;
        Ok(())
    }

    pub fn all_post_ids(&self) -> Result<HashSet<i64>, anyhow::Error> {
        let sql = "select id from post_tombstone";
        let mut stmt = self.storage.conn.prepare_cached(sql)?;
        let mut rows = stmt.query([])?;

        let mut post_ids = HashSet::new();
        while let Some(row) = rows.next()? {
            let post_id: i64 = row.get(0)?;
            post_ids.insert(post_id);
        }
        Ok(post_ids)
    }

    pub fn delete_all(&self) -> Result<(), anyhow::Error> {
        self.storage
            .conn
            .execute("delete from post_tombstone", [])?;
        Ok(())
    }
}

impl<'a> SettingsStorage<'a> {
    pub fn get<T>(&self, name: &str) -> Result<Option<T>, anyhow::Error>
    where
        T: FromSql,
    {
        let sql = "select value from settings where name = :name";
        let mut stmt = self.storage.conn.prepare_cached(sql)?;
        let mut rows = stmt.query(named_params! {":name": name})?;
        match rows.next() {
            Ok(Some(row)) => {
                let value = row.get(0)?;
                Ok(Some(value))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn set<T>(&self, name: &str, value: T) -> Result<(), anyhow::Error>
    where
        T: ToSql,
    {
        let sql = "insert or replace into settings (name, value) values (:name, :value)";

        self.storage.conn.execute(
            sql,
            named_params! {
                ":name": name,
                ":value": value,
            },
        )?;
        Ok(())
    }

    pub fn get_max_page(&self) -> Result<Option<u32>, anyhow::Error> {
        self.get("max_page")
    }

    pub fn set_max_page(&self, max_page: u32) -> Result<(), anyhow::Error> {
        self.set("max_page", &max_page)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    #[test]
    fn test_max_page_settings() {
        let dbfile = Path::new("db.db");
        if dbfile.exists() {
            fs::remove_file(dbfile).unwrap();
        }

        let storage = Storage::open(dbfile).unwrap();
        assert!(storage.settings().get_max_page().unwrap().is_none());
        storage.settings().set_max_page(123).unwrap();
        assert_eq!(storage.settings().get_max_page().unwrap(), Some(123));
    }
}
