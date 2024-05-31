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

// Used for search results
pub struct SearchResult {
    pub id: String,
    pub blob_id: String,
    pub subject: String,
}

// We use lazy_static to ensure that the schema is only built once
// and then reused for all operations. This is a performance optimization.
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

/**
 * Write a document to the index.
 * The document is created using both the JMAP response and the parsed email message.
 * This way the full body of the email message is indexed.
 */
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

    let body_text = message.body_html(0).unwrap_or_default();
    
    doc.add_text(fields["body"], body_text);

    indexer
        .add_document(doc)
        .with_context(|| format!("Error adding document to index"))
}

/**
 * Search the index for the given query, searching in the subject and body fields.
 * Limit to 100 results by default, but allow the limit to be set.
 * Return a vector of search results to be displayed, each result containing the jmap id, blob_id and subject.
 */
pub fn search(folder: String, query: String, limit: Option<usize>) -> anyhow::Result<Vec<SearchResult>> {
    let index = Index::open_in_dir(folder)?;
    let reader = index.reader()?;
    let searcher = reader.searcher();
    let query_parser = QueryParser::for_index(
        &index,
        vec![EMAIL_SCHEMA.fields["subject"], EMAIL_SCHEMA.fields["body"]],
    );
    let query = query_parser.parse_query(&query)?;
    let top_docs = searcher.search(&query, &TopDocs::with_limit(limit.unwrap_or(100)))?;
    let mut docs: Vec<SearchResult> = vec![];

    for (_score, doc_address) in top_docs {
        let doc = searcher.doc(doc_address)?;
        let id = doc
            .get_first(EMAIL_SCHEMA.fields["blob_id"])
            .map(|val| val.as_text())
            .unwrap_or_default()
            .unwrap_or_default()
            .to_string(); // Convert Option<&str> to String

        let blob_id = doc
            .get_first(EMAIL_SCHEMA.fields["blob_id"])
            .map(|val| val.as_text())
            .unwrap_or_default()
            .unwrap_or_default()
            .to_string(); // Convert Option<&str> to String

        let subject = doc
            .get_first(EMAIL_SCHEMA.fields["subject"])
            .map(|val| val.as_text())
            .unwrap_or_default()
            .unwrap_or_default()
            .to_string(); // Convert Option<&str> to String
        
        docs.push(SearchResult { id, blob_id, subject });
    }

    return Ok(docs);
}


// Testing the search module below here
#[cfg(test)]
mod tests {
    use mail_parser::MessageParser;
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn test_create_indexer() {
        // Create an indexer and index a document to a temporary directory
        let temp_dir = TempDir::new().unwrap();
        let path: String = temp_dir.path().to_str().unwrap().to_string();
        let indexer = create_indexer(path);
        
        assert!(indexer.is_ok());
    }

    #[test]
    fn test_write_document() {
        // Create an indexer and index a document to a temporary directory
        let temp_dir = TempDir::new().unwrap();
        let path: String = temp_dir.path().to_str().unwrap().to_string();
        println!("Path: {}", path);
        let mut indexer = create_indexer(path).unwrap();

        // Create email from a JSON string
        let email = serde_json::from_str::<Email>(
            r#"{
    "id": "123",
    "blobId": "456",
    "subject": "Test email",
    "from": [
        {
            "name": "John Doe",
            "email": "test@example.com"
        }
    ],
    "to": [
        {
            "name": "Mary Smith",
            "email": "mary@example.com"
        }
    ]
}
            "#,
        );

        let message = MessageParser::default().parse(
            r#"
From: John Doe <jdoe@machine.example>
To: Mary Smith <mary@example.net>
Subject: Saying Hello
Date: Fri, 21 Nov 1997 09:55:06 -0600
Message-ID: <1234@local.machine.example>

This is a message just to say hello.
So, "Hello".
            "#
        );

        let result = write_document(&indexer, &email.unwrap(), &message.unwrap());
        indexer.commit().unwrap();

        assert!(result.is_ok());
    }

    #[test]
    fn test_search() {
        // Create an indexer and index a document to a temporary directory
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_path_buf();
        let index_path: String = path.to_str().unwrap().to_string();
        let search_path = path.to_str().unwrap().to_string();

        // Create indexer
        let mut indexer = create_indexer(index_path).unwrap();

        // Create email from a JSON string
        let email = serde_json::from_str::<Email>(
            r#"{
    "id": "123",
    "blobId": "456",
    "subject": "Test email",
    "from": [
        {
            "name": "John Doe",
            "email": "test@example.com"
        }
    ],
    "to": [
        {
            "name": "Mary Smith",
            "email": "mary@example.com"
        }
    ]
}
"#,
        );

        let input = br#"Resent-From: Mary Smith <mary@example.net>
Resent-To: Jane Brown <j-brown@other.example>
Resent-Date: Mon, 24 Nov 1997 14:22:01 -0800
Resent-Message-ID: <78910@example.net>
From: John Doe <jdoe@machine.example>
To: Mary Smith <mary@example.net>
Subject: Saying Hello
Date: Fri, 21 Nov 1997 09:55:06 -0600
Message-ID: <1234@local.machine.example>

This is a message just to say hello.
So, "Hello".
"#;

        let message = MessageParser::default().parse(input);

        write_document(&indexer, &email.unwrap(), &message.unwrap()).unwrap();
        indexer.commit().unwrap();
        let results = search(search_path.clone(), "Hello".to_string(), Some(10));
        let no_results = search(search_path.clone(), "Goodbye".to_string(), Some(10));
        
        assert!(results.is_ok());
        assert_eq!(results.unwrap().len(), 1);

        assert!(no_results.is_ok());
        assert_eq!(no_results.unwrap().len(), 0);

    }
}