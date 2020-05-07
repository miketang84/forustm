use cang_jie::{CangJieTokenizer, TokenizerOption, CANG_JIE};
use jieba_rs::Jieba;
use tantivy::{collector::TopDocs, doc, query::QueryParser, schema::*, Index, IndexReader, IndexWriter, Searcher, Document, ReloadPolicy};
use tantivy::directory::MmapDirectory;
use std::{collections::HashSet, io, iter::FromIterator, sync::Arc};
use std::path::Path;
use serde::{Serialize, Deserialize};
use serde_json;
use log::info;
use uuid::Uuid;
use chrono::{DateTime, Utc, NaiveDateTime};
use crossbeam::channel::{Sender, Receiver};
use std::thread;

#[derive(Debug)]
pub struct Doc2Index {
    pub article_id: String,
    pub created_time: String,
    pub title: String,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DocFromIndex {
    pub article_id: Vec<String>,
    pub title: Vec<String>,
    pub created_time: Vec<String>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DocFromIndexOuter {
    pub article_id: Uuid,
    pub title: String,
    pub created_time: DateTime<Utc>
}

pub struct TantivyIndex {
    // pub index: Index,
    pub schema: Schema,
    pub reader: IndexReader,
    pub writer: IndexWriter,
    pub query_parser: QueryParser
}

pub enum TanAction {
    Add,
    Update,
    Delete,
    Query
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
    schema_builder.add_text_field("created_time", STRING | STORED);
    schema_builder.add_text_field("title", text_options);
    schema_builder.add_text_field("content", text_options_nostored);
    let schema = schema_builder.build();

    let index = Index::open_or_create(MmapDirectory::open(Path::new("search_index/")).unwrap(), schema.clone())?;
    // let index = Index::open(MmapDirectory::open(Path::new("search_index/")).unwrap())?;
    index.tokenizers().register(CANG_JIE, tokenizer()); // Build cang-jie Tokenizer

    let writer = index.writer(50 * 1024 * 1024)?;

    let title = schema.get_field("title").unwrap();
    let content = schema.get_field("content").unwrap();

    let query_parser = QueryParser::for_index(&index, vec![title, content]);

    let reader = index
	.reader_builder()
	.reload_policy(ReloadPolicy::OnCommit)
	.try_into()?;

    Ok(TantivyIndex {
	schema: index.schema(),
	reader,
	writer,
	query_parser
    })
}

impl TantivyIndex {

    pub fn add_doc(&mut self, doc: Doc2Index) -> tantivy::Result<()> {
	let schema = &self.schema;

	let article_id = schema.get_field("article_id").unwrap();
	let created_time = schema.get_field("created_time").unwrap();
	let title = schema.get_field("title").unwrap();
	let content = schema.get_field("content").unwrap();

	let mut a_doc = Document::default();
	a_doc.add_text(article_id, &doc.article_id);
	a_doc.add_text(created_time, &doc.created_time);
	a_doc.add_text(title, &doc.title);
	a_doc.add_text(content, &doc.content);

	self.writer.add_document(a_doc);

	self.writer.commit()?;

	info!("add to tantivy index {:?}", doc.article_id);

	Ok(())

    }

    pub fn update_doc(&mut self, doc: Doc2Index) -> tantivy::Result<()> {
	let schema = &self.schema;
	let article_id = schema.get_field("article_id").unwrap();
	let _n = self.writer.delete_term(Term::from_field_text(article_id, &doc.article_id));

	self.writer.commit()?;

	self.add_doc(doc)
    }

    pub fn delete_doc(&mut self, doc_id: &str) -> tantivy::Result<()> {
	let schema = &self.schema;
	let article_id = schema.get_field("article_id").unwrap();
	let _n = self.writer.delete_term(Term::from_field_text(article_id, doc_id));

	self.writer.commit()?;

	Ok(())
    }

    pub fn query(&self, s: &str) -> tantivy::Result<Vec<DocFromIndexOuter>> {
	let schema = &self.schema;

	// self.index.load_searchers()?;
	// let searcher = self.index.searcher();
	let searcher = self.reader.searcher();

	let q = self.query_parser.parse_query(s)?;

	let mut top_docs = TopDocs::with_limit(50);

	let doc_addresses = searcher.search(&q, &mut top_docs)?;

	let mut r_vec: Vec<DocFromIndexOuter> = vec![];
	for (_, doc_address) in doc_addresses {
	    let retrieved_doc = searcher.doc(doc_address)?;
	    let json_str = schema.to_json(&retrieved_doc);
	    let doc_from_index: DocFromIndex = serde_json::from_str(&json_str).unwrap();

	    info!("{:?}", doc_from_index);
	    let created_timestamp: i64 = doc_from_index.created_time[0].parse::<i64>().unwrap();

	    let new_doc = DocFromIndexOuter {
		article_id: doc_from_index.article_id[0].parse::<Uuid>().unwrap(),
		created_time: DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(created_timestamp, 0), Utc),
		title: doc_from_index.title[0].to_owned()
	    };

	    r_vec.push(new_doc);
	}

	r_vec.sort_by(|a, b| b.created_time.cmp(&a.created_time));

	Ok(r_vec)
    }
}



fn tokenizer() -> CangJieTokenizer {
    CangJieTokenizer {
	worker: Arc::new(Jieba::empty()), // empty dictionary
	option: TokenizerOption::Unicode,
    }
}


pub fn run_tantivy(tan_index_rx: Receiver<(TanAction, String, Option<Doc2Index>)>, tan_query_tx: Sender<Vec<DocFromIndexOuter>>) {

    let mut ttv_index = match init() {
       Ok(ttv_index) => {
	   //Arc::new(Mutex::new(ttv_index))
	   ttv_index
       },
       Err(e) => {
	   panic!("{:?}", e);
       }
    };

    thread::spawn(move || {
	loop {
	    match tan_index_rx.recv() {
		Ok((action, head_data, doc)) => {
		    match action {
			TanAction::Add => {
			    if doc.is_some() {
				let _ = ttv_index.add_doc(doc.unwrap());
			    }
			},
			TanAction::Update => {
			    if doc.is_some() {
				let _ = ttv_index.update_doc(doc.unwrap());
			    }
			},
			TanAction::Delete => {
			    let _ = ttv_index.delete_doc(&head_data);
			},
			TanAction::Query => {
			    let docs = ttv_index.query(&head_data).unwrap_or(Vec::new());
			    let _ = tan_query_tx.send(docs);
			}
		    }
		},
		Err(_) => {}
	    }
	}
    });

}
