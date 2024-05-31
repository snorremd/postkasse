use std::collections::HashMap;

use anyhow::Context;
use opendal::{layers::RetryLayer, Operator, Scheme};

/**
 * Create a storage backend with the given configuration.
 * Exit the process if the backend cannot be created.
 * Handle exit here to avoid having to handle anyhow::Result in main 
 */
pub fn create_storage_backend(scheme: Scheme, config: HashMap<String, String>) -> anyhow::Result<Operator> {
    let operator = Operator::via_map(scheme, config);

    let retry_operator = operator
        .with_context(|| "Error creating storage backend")?
        .layer(RetryLayer::new()); // Apply retry layer to avoid transient errors

    Ok(retry_operator)
}

