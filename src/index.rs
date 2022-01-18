use crate::weibo::post::Post;
use std::path::Path;
use tantivy::collector::TopDocs;
use tantivy::directory::MmapDirectory;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::{Index, ReloadPolicy};

pub struct WeiboIndexer {
    index: Index,
}

impl WeiboIndexer {
    pub fn with_index_dir<P: AsRef<Path>>(dir: P) -> Result<WeiboIndexer, anyhow::Error> {
        let mut schema_builder = Schema::builder();
        schema_builder.add_text_field("url", STRING | STORED);
        schema_builder.add_text_field("user", STRING | STORED);
        schema_builder.add_text_field("text", TEXT | STORED);
        schema_builder.add_u64_field("media_type", IntOptions::default().set_indexed());
        let schema = schema_builder.build();

        let dir = MmapDirectory::open(dir)?;
        let index = Index::open_or_create(dir, schema)?;
        Ok(WeiboIndexer { index })
    }

    pub fn schema(&self) -> Schema {
        self.index.schema()
    }

    pub fn index_weibo_posts(&self, posts: &[Post]) -> Result<(), anyhow::Error> {
        let mut index_writer = self.index.writer(50_000_000)?;
        let schema = self.schema();

        for post in posts {
            let mut doc = Document::default();
            doc.add_text(schema.get_field("url").unwrap(), post.url());
            doc.add_text(schema.get_field("user").unwrap(), &post.user.screen_name);
            doc.add_text(schema.get_field("text").unwrap(), &post.text_raw);
            doc.add_u64(
                schema.get_field("media_type").unwrap(),
                post.media_type() as u8 as u64,
            );
            index_writer.add_document(doc);
        }
        index_writer.commit()?;
        Ok(())
    }

    pub fn search(
        &self,
        params: &WeiboSearchParams,
        limit: usize,
    ) -> Result<Vec<String>, anyhow::Error> {
        let mut query_str = String::new();
        if let Some(query) = &params.query {
            let text_query = format!("text:{}", query);
            query_str.push_str(&text_query);
        }
        if let Some(media_type) = params.media_type {
            let media_query = format!("media_type:{}", media_type);
            query_str.push_str(&media_query);
        }
        let query_parser = QueryParser::for_index(&self.index, vec![]);
        let query = query_parser.parse_query(&query_str)?;

        let reader = self
            .index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommit)
            .try_into()?;
        let searcher = reader.searcher();
        let top_docs = searcher.search(&query, &TopDocs::with_limit(limit))?;

        let schema = self.schema();
        let mut res = vec![];
        for (_score, doc_address) in top_docs {
            let retrieved_doc = searcher.doc(doc_address)?;
            res.push(schema.to_json(&retrieved_doc));
        }

        Ok(res)
    }
}

pub struct WeiboSearchParams {
    pub media_type: Option<u8>,
    pub query: Option<String>,
}
