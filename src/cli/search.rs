use log::{error, info};
use console::style;
use prettytable::{format, Cell, Row, Table};

use crate::conf::Search;
use crate::core::search::search;

pub fn search_emails(search_conf: Search, query: String, limit: Option<usize>, fields: Option<Vec<String>>) {
    if !search_conf.enable {
        let err = format!("Search is not enabled in config");
        error!("{}", style(err).red().bold());
        std::process::exit(1);
    } else {
        let mut table = Table::new();
        let mut header = Row::empty();
        let fields = fields.unwrap_or_default();

        for field in fields.iter() {
            header.add_cell(Cell::new(&field));
        }

        table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
        table.set_titles(header);

        
        let result = search(search_conf.folder, query, limit).unwrap_or_else(|_e| {
            let err = format!("Could not search index");
            error!("{}", style(err).red().bold());
            std::process::exit(1);
        });

        info!("Number of results: {}", result.len());
        for doc in result {
            let mut result = Row::empty();
            for field in fields.iter() {
                match field.as_str() {
                    "id" => result.add_cell(Cell::new(&doc.id)),
                    "blob_id" => result.add_cell(Cell::new(&doc.blob_id)),
                    "subject" => result.add_cell(Cell::new(&doc.subject)),
                    _ => (),
                }
            }
            table.add_row(result);
        }

        table.printstd();
    }
}