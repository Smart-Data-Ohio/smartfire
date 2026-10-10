//! The Jbuilder views: `messages/_message.json`, `messages/by_bots/{index,show}.json`,
//! `messages/boosts/_boost.json` and `messages/boosts/by_bots/show.json`. Field order is the
//! JSON key order Jbuilder emits. Serialize with [`crate::helpers::to_rails_json`].


// A Jbuilder fragment's payload is its JSON.
impl crate::fragment_cache::CacheSize for UserJson {
    fn cache_size(&self) -> usize {
        crate::fragment_cache::serialized_size(self)
    }
}

impl crate::fragment_cache::CacheSize for MessageJson {
    fn cache_size(&self) -> usize {
        crate::fragment_cache::serialized_size(self)
    }
}

impl crate::fragment_cache::CacheSize for BoostJson {
    fn cache_size(&self) -> usize {
        crate::fragment_cache::serialized_size(self)
    }
}
pub use campfire_presentation::messages::json::*;
