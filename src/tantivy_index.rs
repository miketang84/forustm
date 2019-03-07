use cang_jie::{CangJieTokenizer, TokenizerOption, CANG_JIE};
use jieba_rs::Jieba;
use tantivy::{collector::TopDocs, doc, query::QueryParser, schema::*, Index, IndexWriter, Searcher, Document};
use tantivy::directory::MmapDirectory;
use std::{collections::HashSet, io, iter::FromIterator, sync::Arc};
use std::path::Path;

pub struct Doc2Index {
    pub article_id: String,
    pub title: String,
    pub content: String,
}

pub struct TantivyIndex {
    pub index: Index,
    pub writer: IndexWriter,
    pub query_parser: QueryParser
}



pub fn init() -> tantivy::Result<TantivyIndex> {

    let mut schema_builder = SchemaBuilder::default();

    let text_indexing = TextFieldIndexing::default()
        .set_tokenizer(CANG_JIE) // Set custom tokenizer
        .set_index_option(IndexRecordOption::WithFreqsAndPositions);
    let text_options = TextOptions::default()
        .set_indexing_options(text_indexing.clone())
        .set_stored();
    let text_options_nostored = TextOptions::default()
        .set_indexing_options(text_indexing);

    schema_builder.add_text_field("article_id", STRING | STORED);
    schema_builder.add_text_field("title", text_options);
    schema_builder.add_text_field("content", text_options_nostored);
    let schema = schema_builder.build();

    let index = Index::create(MmapDirectory::open(Path::new("search_index")).unwrap(), schema.clone())?;
    index.tokenizers().register(CANG_JIE, tokenizer()); // Build cang-jie Tokenizer

    let writer = index.writer(50 * 1024 * 1024)?;

    let title = schema.get_field("title").unwrap();
    let content = schema.get_field("content").unwrap();

    let query_parser = QueryParser::for_index(&index, vec![title, content]);

    Ok(TantivyIndex {
        index,
        writer,
        query_parser
    })

}

impl TantivyIndex {

    pub fn add_doc(&mut self, doc: Doc2Index) -> tantivy::Result<()> {
        let schema = self.index.schema();

        let article_id = schema.get_field("article_id").unwrap();
        let title = schema.get_field("title").unwrap();
        let content = schema.get_field("content").unwrap();

        let mut a_doc = Document::default();
        a_doc.add_text(article_id, &doc.article_id);
        a_doc.add_text(title, &doc.title);
        a_doc.add_text(content, &doc.content);
        
        self.writer.add_document(a_doc);

        self.writer.commit()?;

        Ok(())

    }

    pub fn update_doc(&mut self, doc: Doc2Index) -> tantivy::Result<()> {
        let schema = self.index.schema();
        let article_id = schema.get_field("article_id").unwrap();
        let _n = self.writer.delete_term(Term::from_field_text(article_id, &doc.article_id));

        self.writer.commit()?;

        self.add_doc(doc)
    }

    pub fn delete_doc(&mut self, doc_id: &str) -> tantivy::Result<()> {
        let schema = self.index.schema();
        let article_id = schema.get_field("article_id").unwrap();
        let _n = self.writer.delete_term(Term::from_field_text(article_id, doc_id));

        self.writer.commit()?;

        Ok(())
    }

    pub fn query(&self, s: &str) -> tantivy::Result<Vec<String>> {
        let schema = self.index.schema();

        self.index.load_searchers()?;
        let searcher = self.index.searcher();

        let q = self.query_parser.parse_query(s)?;

        let mut top_docs = TopDocs::with_limit(50);

        let doc_addresses = searcher.search(&q, &mut top_docs)?;

        let mut r_vec: Vec<String> = vec![];
        for (_, doc_address) in doc_addresses {
            let retrieved_doc = searcher.doc(doc_address)?;
            r_vec.push(schema.to_json(&retrieved_doc));
            
            println!("{}", schema.to_json(&retrieved_doc));
        }

        // self.writer.wait_merging_threads()?;

        Ok(r_vec)
    }
}



fn tokenizer() -> CangJieTokenizer {
    CangJieTokenizer {
        worker: Arc::new(Jieba::empty()), // empty dictionary
        option: TokenizerOption::Unicode,
    }
}

