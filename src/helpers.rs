use jmap_client::client::Client;

// Borrow client to get max_objects_in_get, return usize
pub fn max_objects_in_get(client: &Client) -> usize {
    // Return min of 100 or max_objects_in_get
    client.session().core_capabilities().map(|c| c.max_objects_in_get()).unwrap_or(50).min(50)
}