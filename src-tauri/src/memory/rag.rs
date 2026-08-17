//! Retrieval-Augmented Generation (Phase 3.3 — v0.5).
//!
//! Given a user query, pull the top-K semantically similar knowledge
//! entries from the [`EmbeddingStore`] and inject them as a system-prompt
//! fragment so the AI's next reply is grounded in the user's stored facts.
//!
//! This closes the v0.3 → v0.4 → v0.5 RAG loop:
//! - v0.3: AI stores facts via `memory_remember` (key/value).
//! - v0.4: `memory_search` tool exposed Jaccard token-overlap search.
//! - v0.5: vector-embedding-based search runs *automatically* before
//!   every chat, injecting the relevant facts into the system prompt.
//!
//! The agent loop (`ai::agent`) calls [`inject_rag_context`] before
//! building the ChatRequest. If the knowledge base is empty, RAG is a
//! no-op (no extra tokens, no AI call).

use crate::ai::provider::ChatMessage;
use crate::error::Result;
use crate::memory::embeddings::EmbeddingStore;
use crate::memory::KnowledgeEntry;

/// Default max number of facts to inject per chat turn.
pub const DEFAULT_RAG_TOP_K: usize = 5;

/// Min cosine similarity score for an entry to be considered "relevant".
/// 0.30 is empirically a good floor for our trigram-hash embeddings.
pub const DEFAULT_RAG_MIN_SCORE: f32 = 0.30;

/// The injected system-prompt fragment, prefixed onto the conversation
/// when at least one relevant fact is found.
pub const RAG_SYSTEM_PREFIX: &str = "--- Retrieved from your knowledge base ---\n";

/// Append a RAG fragment to the first system message in `messages`.
///
/// If `messages` is empty or the first message is not a system message,
/// we prepend a new system message containing only the RAG fragment.
/// If no relevant facts are found, `messages` is left unchanged.
///
/// Returns the number of facts actually injected.
pub fn inject_rag_context(
    messages: &mut Vec<ChatMessage>,
    query: &str,
    store: &EmbeddingStore,
    top_k: usize,
    min_score: f32,
) -> Result<usize> {
    if query.trim().is_empty() {
        return Ok(0);
    }
    let hits = store.search(query, top_k)?;
    let filtered: Vec<&KnowledgeEntry> = hits
        .iter()
        .filter(|(s, _)| *s >= min_score)
        .map(|(_, e)| e)
        .collect();
    if filtered.is_empty() {
        return Ok(0);
    }
    let mut block = String::from(RAG_SYSTEM_PREFIX);
    for entry in &filtered {
        block.push_str(&format!(
            "- {} = {} (confidence: {:.2})\n",
            entry.key, entry.value, entry.confidence
        ));
    }
    block.push('\n');
    block.push_str(
        "Use these facts to inform your reply. If a fact contradicts the user's \
         current message, defer to the user. Do not mention that you retrieved \
         facts from a knowledge base — just use them naturally.",
    );

    // Find the first system message and prepend the RAG block to it,
    // so the AI sees both the role description and the facts up-front.
    if let Some(first) = messages.first_mut() {
        if matches!(first.role, crate::ai::provider::Role::System) {
            let new_content = format!("{block}\n\n{}", first.content);
            first.content = new_content;
            return Ok(filtered.len());
        }
    }
    // No system message at the front — insert one.
    messages.insert(0, ChatMessage::system(block));
    Ok(filtered.len())
}

/// Convenience wrapper: inject up to `DEFAULT_RAG_TOP_K` facts with the
/// default min-score threshold.
pub fn inject_default(
    messages: &mut Vec<ChatMessage>,
    query: &str,
    store: &EmbeddingStore,
) -> Result<usize> {
    inject_rag_context(messages, query, store, DEFAULT_RAG_TOP_K, DEFAULT_RAG_MIN_SCORE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::provider::{ChatMessage, Role};
    use crate::memory::store::MemoryStore;

    fn open_test_store() -> MemoryStore {
        MemoryStore::open_in_memory().expect("in-memory sqlite should always open")
    }

    #[test]
    fn rag_no_facts_returns_zero() {
        let store = open_test_store();
        let mut msgs = vec![ChatMessage::system("You are Aegis AI.")];
        let n = inject_default(&mut msgs, "what's my dog's name", &store.embeddings).unwrap();
        assert_eq!(n, 0);
        assert_eq!(msgs.len(), 1);
    }

    #[test]
    fn rag_injects_relevant_facts() {
        let store = open_test_store();
        store.knowledge.remember("pet_name", "Rex", Some("user"), 0.9).unwrap();
        store.knowledge.remember("favorite_color", "blue", Some("user"), 0.9).unwrap();
        store.embeddings.upsert("pet_name", "pet_name Rex dog").unwrap();
        store.embeddings.upsert("favorite_color", "favorite_color blue").unwrap();

        let mut msgs = vec![ChatMessage::system("You are Aegis AI.")];
        let n = inject_default(&mut msgs, "what is my dog's name?", &store.embeddings).unwrap();
        assert!(n >= 1, "expected at least one injected fact, got {n}");
        let sys = msgs.first().unwrap();
        assert!(matches!(sys.role, Role::System));
        assert!(sys.content.contains("Rex") || sys.content.contains("pet_name"));
        assert!(sys.content.contains(RAG_SYSTEM_PREFIX));
    }

    #[test]
    fn rag_skips_below_threshold() {
        let store = open_test_store();
        store.knowledge.remember("foo", "bar", Some("user"), 0.5).unwrap();
        store.embeddings.upsert("foo", "foo bar").unwrap();
        let mut msgs = vec![ChatMessage::system("hi")];
        // The query is completely unrelated; with the trigram embedding,
        // there should be very low similarity — but if our min-score is 0.99,
        // we should skip every entry.
        let n = inject_rag_context(&mut msgs, "completely different topic", &store.embeddings, 5, 0.99).unwrap();
        assert_eq!(n, 0);
    }

    #[test]
    fn rag_inserts_system_if_missing() {
        let store = open_test_store();
        store.knowledge.remember("name", "Alice", Some("user"), 1.0).unwrap();
        store.embeddings.upsert("name", "name Alice").unwrap();
        let mut msgs = vec![ChatMessage::user("what's my name?")];
        let n = inject_default(&mut msgs, "what is my name?", &store.embeddings).unwrap();
        assert_eq!(n, 1);
        assert_eq!(msgs.len(), 2);
        assert!(matches!(msgs[0].role, Role::System));
    }
}
