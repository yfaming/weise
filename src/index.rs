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
        let jieba_tokenizer = tantivy_jieba::JiebaTokenizer {};
        let text_field_indexing = TextFieldIndexing::default()
            .set_tokenizer("jieba")
            .set_index_option(IndexRecordOption::WithFreqsAndPositions);
        let text_options = TextOptions::default()
            .set_indexing_options(text_field_indexing)
            .set_stored();

        let mut schema_builder = Schema::builder();
        schema_builder.add_text_field("url", STRING | STORED);
        schema_builder.add_text_field("user", STRING | STORED);
        schema_builder.add_text_field("text", text_options.clone());
        schema_builder.add_u64_field("media_type", IntOptions::default().set_indexed());
        schema_builder.add_text_field("retweeted_user", STRING | STORED);
        schema_builder.add_text_field("retweeted_text", text_options.clone());
        let schema = schema_builder.build();

        let dir = MmapDirectory::open(dir)?;
        let index = Index::open_or_create(dir, schema)?;
        index.tokenizers().register("jieba", jieba_tokenizer);
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
            if let Some(retweeted_post) = &post.retweeted_post {
                doc.add_text(
                    schema.get_field("retweeted_user").unwrap(),
                    &retweeted_post.user.screen_name,
                );
                doc.add_text(
                    schema.get_field("retweeted_text").unwrap(),
                    &retweeted_post.text_raw,
                );
            }

            index_writer.add_document(doc);
        }
        index_writer.commit()?;
        Ok(())
    }

    pub fn search(
        &self,
        params: &WeiboSearchParams,
        limit: usize,
    ) -> Result<Vec<SearchedWeiboPost>, anyhow::Error> {
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

        let mut posts = vec![];
        let schema = self.schema();
        for (_score, doc_address) in top_docs {
            let retrieved_doc = searcher.doc(doc_address)?;
            posts.push(SearchedWeiboPost::from_doc(&schema, &retrieved_doc));
        }

        Ok(posts)
    }
}

pub struct WeiboSearchParams {
    pub media_type: Option<u8>,
    pub query: Option<String>,
}

pub struct SearchedWeiboPost {
    pub url: String,
    pub user: String,
    pub text: String,
    pub retweeted_user: Option<String>,
    pub retweeted_text: Option<String>,
}

impl SearchedWeiboPost {
    pub fn from_doc(schema: &Schema, doc: &Document) -> SearchedWeiboPost {
        use std::collections::HashMap;

        let mut field_values = HashMap::new();
        for field_value in doc.field_values() {
            let field_name = schema.get_field_name(field_value.field()).to_string();
            field_values.insert(field_name, field_value.value());
        }

        let url = field_values["url"].text().unwrap().to_string();
        let user = field_values["user"].text().unwrap().to_string();
        let text = field_values["text"].text().unwrap().to_string();

        let retweeted_user = match field_values.get("retweeted_user") {
            None => None,
            Some(retweeted_user) => retweeted_user
                .text()
                .map(|retweeted_user| retweeted_user.to_string()),
        };
        let retweeted_text = match field_values.get("retweeted_text") {
            None => None,
            Some(retweeted_text) => retweeted_text
                .text()
                .map(|retweeted_text| retweeted_text.to_string()),
        };

        SearchedWeiboPost {
            url,
            user,
            text,
            retweeted_user,
            retweeted_text,
        }
    }
}
