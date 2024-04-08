use std::collections::HashMap;

use anyhow::Context;
use jmap_client::email::Email;
use mail_parser::Message;
use tantivy::{
    collector::TopDocs,
    directory::MmapDirectory,
    query::QueryParser,
    schema::{Field, Schema, STORED, TEXT},
    Document, Index, IndexWriter,
};

struct EmailSchema<'a> {
    // Map of string field names to field types
    fields: HashMap<&'a str, Field>,
    schema: Schema,
}

lazy_static! {
    static ref EMAIL_SCHEMA: EmailSchema<'static> = schema_builder();
}

fn schema_builder() -> EmailSchema<'static> {
    let mut schema_builder = Schema::builder();
    let id = schema_builder.add_text_field("id", TEXT | STORED);
    let blob_id = schema_builder.add_text_field("blob_id", TEXT | STORED);
    let subject = schema_builder.add_text_field("subject", TEXT | STORED);
    let from_name = schema_builder.add_text_field("from_name", TEXT | STORED);
    let from_email = schema_builder.add_text_field("from_email", TEXT | STORED);
    let to_name = schema_builder.add_text_field("to_name", TEXT | STORED);
    let to_email = schema_builder.add_text_field("to_email", TEXT | STORED);
    let cc_name = schema_builder.add_text_field("cc_name", TEXT);
    let cc_email = schema_builder.add_text_field("cc_email", TEXT);
    let bcc = schema_builder.add_text_field("bcc", TEXT);
    let body = schema_builder.add_text_field("body", TEXT);

    EmailSchema {
        fields: vec![
            ("id", id),
            ("blob_id", blob_id),
            ("subject", subject),
            ("from_name", from_name),
            ("from_email", from_email),
            ("to_name", to_name),
            ("to_email", to_email),
            ("cc_name", cc_name),
            ("cc_email", cc_email),
            ("bcc", bcc),
            ("body", body),
        ]
        .into_iter()
        .collect(),
        schema: schema_builder.build(),
    }
}

pub fn create_indexer(folder: String) -> anyhow::Result<IndexWriter> {
    // Ensure folder exists, if not create it
    std::fs::create_dir_all(&folder)
        .with_context(|| format!("Error creating folder {}", folder))?;

    let schema = EMAIL_SCHEMA.schema.clone();
    let directory = MmapDirectory::open(folder)?;
    let index = Index::open_or_create(directory, schema.clone())?;
    let indexer = index.writer(50_000_000)?;
    return Ok(indexer);
}

pub fn write_document(
    indexer: &IndexWriter,
    email: &Email,
    message: &Message,
) -> anyhow::Result<u64> {
    let fields = &EMAIL_SCHEMA.fields;
    let mut doc = Document::new();
    doc.add_text(fields["id"], email.id().unwrap());
    doc.add_text(fields["blob_id"], email.blob_id().unwrap());
    doc.add_text(fields["subject"], email.subject().unwrap_or_default());

    for from in email.from().unwrap_or_default() {
        doc.add_text(fields["from_name"], from.name().unwrap_or_default());
        doc.add_text(fields["from_email"], from.email());
    }

    for to in email.to().unwrap_or_default() {
        doc.add_text(fields["to_email"], to.email());
        doc.add_text(fields["to_name"], to.name().unwrap_or_default());
    }

    for cc in email.cc().unwrap_or_default() {
        doc.add_text(fields["cc_email"], cc.email());
        doc.add_text(fields["cc_name"], cc.name().unwrap_or_default());
    }

    let body_text = message.body_text(0).unwrap_or_default();
    doc.add_text(fields["body"], body_text);

    indexer
        .add_document(doc)
        .with_context(|| format!("Error adding document to index"))
}

pub fn search(folder: String, query: String) -> anyhow::Result<Vec<String>> {
    let schema = EMAIL_SCHEMA.schema.clone();
    let index = Index::open_in_dir(folder)?;
    let reader = index.reader()?;
    let searcher = reader.searcher();
    let query_parser = QueryParser::for_index(
        &index,
        vec![EMAIL_SCHEMA.fields["subject"], EMAIL_SCHEMA.fields["body"]],
    );
    let query = query_parser.parse_query(&query)?;
    let top_docs = searcher.search(&query, &TopDocs::with_limit(100))?;
    let mut docs = vec![];

    for (_score, doc_address) in top_docs {
        let doc = searcher.doc(doc_address)?;
        docs.push(schema.to_json(&doc));
    }

    return Ok(docs);
}
