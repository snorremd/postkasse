use std::collections::HashMap;
use jmap_client::{client::Client, mailbox::Mailbox};
use petgraph::{algo::toposort, graph::DiGraph};

// Borrow client to get max_objects_in_get, return usize
pub fn max_objects_in_get(client: &Client) -> usize {
    // Return min of 100 or max_objects_in_get
    client.session().core_capabilities().map(|c| c.max_objects_in_get()).unwrap_or(50).min(50)
}

pub fn sort_mailboxes(mailboxes: Vec<Mailbox>) -> anyhow::Result<Vec<Mailbox>> {
    // Sort the mailboxes so that any parent mailboxes are restored first
    // Essentially if a is parent of b, and b is parent of c, then a should be restored first, then b, then c
    // Each mailbox can be parent to multiple mailboxes
    let mut graph = DiGraph::new();
    let mut node_indices = HashMap::new();
    
    // Add nodes to the graph
    for mailbox in mailboxes.iter() {
        node_indices.insert(mailbox.id().unwrap_or_default(), graph.add_node(mailbox.id().unwrap()));
    }

    // Add edges to the graph
    for mailbox in mailboxes.iter() {
        if let Some(parent_id) = mailbox.parent_id() {
            if let Some(&parent_index) = node_indices.get(parent_id) {
                let child_index = *node_indices.get(mailbox.id().unwrap_or_default()).unwrap();
                graph.add_edge(parent_index, child_index, ());
            }
        }
    }

    let sorted_indices = toposort(&graph, None)
        .map_err(|e| anyhow::anyhow!("Error sorting mailboxes: cycle detected at node {:?}", e.node_id()))?;
    
    // Map the sorted indices back to mailboxes
    let mut sorted_mailboxes = Vec::new();
    for index in sorted_indices {
        let id = graph.node_weight(index).unwrap();
        let mailbox = mailboxes.iter().find(|m| m.id().unwrap_or_default() == *id).unwrap();
        sorted_mailboxes.push(mailbox.clone());
    }

    Ok(sorted_mailboxes)
}