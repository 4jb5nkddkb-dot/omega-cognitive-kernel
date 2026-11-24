#[allow(unused_imports)]
use crate::prelude::*;

pub mod crypto;
pub mod token_book;
#[cfg(feature = "std")]
pub mod vault;

pub use token_book::{ActionBoundary, AuthorizationToken, TokenBook};
#[cfg(feature = "std")]
pub use vault::{StorageLock, Vault};

pub fn default_constraints() -> hashbrown::HashMap<String, f64> {
    let mut c = hashbrown::HashMap::new();
    c.insert("max_risk".into(), 0.65);
    c.insert("max_actions_per_step".into(), 3.0);
    c
}
