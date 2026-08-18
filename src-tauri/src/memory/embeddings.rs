//! Vector embeddings for the knowledge base (Phase 3.3 — v0.5).
//!
//! A lightweight, zero-dependency vector store for retrieval-augmented
//! generation. We deliberately avoid pulling in `qdrant`, `lancedb`, or
//! `ort` + ONNX models in v0.5 — those would require downloading model
//! weights at build/install time and would break the release pipeline's
//! reproducibility.
//!
//! Instead, each knowledge entry is hashed into a 256-dim sparse vector
//! using a character-trigram hashing trick. Cosine similarity over these
//! vectors is a poor man's semantic search but:
//! - It's deterministic, fast, and SQLite-storable.
//! - It handles typos and morphological variants better than the v0.4
//!   Jaccard token-overlap baseline.
//! - Phase 4 will swap in a real embedding model (all-MiniLM-L6-v2 via
//!   `ort`) without changing the storage interface.

use std::collections::HashMap;

use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::error::Result;

use super::knowledge::KnowledgeEntry;
use super::store::SharedConn;

/// Dimensionality of the hash-bucketed vector.
///
/// 256 buckets × 4 bytes/value = 1 KB per embedding. 10k entries × 1 KB =
/// 10 MB — fits comfortably in SQLite without bloating the page cache.
pub const EMBED_DIM: usize = 256;

/// A fixed-dim vector. We use `Vec<f32>` instead of a `[f32; 256]` so we
/// can store variable-dim experimental embeddings during dev.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Embedding {
    pub dim: usize,
    pub values: Vec<f32>,
}

impl Embedding {
    pub fn zeros(dim: usize) -> Self {
        Self {
            dim,
            values: vec![0.0; dim],
        }
    }

    /// Cosine similarity in `[-1, 1]`. Returns `0.0` if either vector is
    /// all-zeros or dimensions differ.
    pub fn cosine(&self, other: &Embedding) -> f32 {
        if self.dim != other.dim || self.dim == 0 {
            return 0.0;
        }
        let mut dot = 0.0f32;
        let mut a_norm = 0.0f32;
        let mut b_norm = 0.0f32;
        for i in 0..self.dim {
            dot += self.values[i] * other.values[i];
            a_norm += self.values[i] * self.values[i];
            b_norm += other.values[i] * other.values[i];
        }
        if a_norm <= 0.0 || b_norm <= 0.0 {
            return 0.0;
        }
        dot / (a_norm.sqrt() * b_norm.sqrt())
    }
}

/// Hash a string into a sparse vector using character-trigram feature
/// hashing plus word-unigram features. Each trigram and each word is
/// hashed into one of `dim` buckets; the bucket value is the count of
/// features that landed there.
///
/// Combining char-trigrams (good for typos and morphology) with word
/// unigrams (good for semantic concepts like "dog" matching "dog")
/// produces a meaningfully better retrieval quality than trigrams alone,
/// which is what the v0.5 RAG pipeline needs.
///
/// Word features are only added when the input contains at least two
/// words. Single-word inputs (e.g. "calendar" vs "calender") rely on
/// trigram overlap alone — that's where typo tolerance matters most and
/// word features would only dilute the signal.
pub fn embed_text(text: &str, dim: usize) -> Embedding {
    let mut v = vec![0.0f32; dim];
    let lower = text.to_lowercase();
    let chars: Vec<char> = lower.chars().collect();
    if chars.len() < 2 {
        // For very short inputs, hash unigrams.
        for c in &chars {
            let h = fxhash_char(*c) as usize % dim;
            v[h] += 1.0;
        }
    } else {
        // Walk trigrams. Pad with spaces so the first/last chars are
        // included in some trigram.
        let mut padded = vec![' '];
        padded.extend(chars.iter().copied());
        padded.push(' ');
        for w in padded.windows(3) {
            let s: String = w.iter().collect();
            let h = fxhash_str(&s) as usize % dim;
            v[h] += 1.0;
        }
        // For multi-word inputs, also hash word unigrams. Word-level
        // features give "dog" in the query a chance to match "dog" in
        // the stored fact directly, even when the surrounding
        // characters don't line up into shared trigrams.
        let words: Vec<&str> = lower
            .split(|c: char| c.is_whitespace() || !c.is_alphanumeric())
            .filter(|s| s.len() >= 3)
            .collect();
        if words.len() >= 2 {
            for word in words {
                let h = fxhash_str(word) as usize % dim;
                v[h] += 2.0; // weight words slightly higher than trigrams
            }
        }
    }
    // L2 normalize so cosine is just a dot product. Skip if all-zero.
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for x in v.iter_mut() {
            *x /= norm;
        }
    }
    Embedding { dim, values: v }
}

/// Tiny deterministic string hash (FNV-1a 32-bit).
fn fxhash_str(s: &str) -> u32 {
    let mut h: u32 = 0x811c9dc5;
    for &b in s.as_bytes() {
        h ^= b as u32;
        h = h.wrapping_mul(0x0100_0193);
    }
    h
}

fn fxhash_char(c: char) -> u32 {
    let mut buf = [0u8; 4];
    let s = c.encode_utf8(&mut buf);
    fxhash_str(s)
}

/// SQLite-backed embedding store. One row per knowledge key.
pub struct EmbeddingStore {
    conn: SharedConn,
}

impl EmbeddingStore {
    pub fn new(conn: SharedConn) -> Self {
        Self { conn }
    }

    /// Insert or update the embedding for a given knowledge key.
    pub fn upsert(&self, key: &str, text: &str) -> Result<()> {
        let emb = embed_text(text, EMBED_DIM);
        let blob: Vec<u8> = emb.values.iter().flat_map(|f| f.to_le_bytes()).collect();
        let now_ms = time::OffsetDateTime::now_utc().unix_timestamp() as u64 * 1000;
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO knowledge_embeddings (key, dim, vector, source_text, updated_at_ms)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(key) DO UPDATE SET vector=?3, source_text=?4, updated_at_ms=?5",
            params![key, emb.dim as i64, blob, text, now_ms as i64],
        )?;
        Ok(())
    }

    /// Delete an embedding by key.
    pub fn delete(&self, key: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "DELETE FROM knowledge_embeddings WHERE key=?1",
            params![key],
        )?;
        Ok(())
    }

    /// Search for the top-K knowledge entries whose embeddings are most
    /// similar to `query` (cosine). Returns `(score, KnowledgeEntry)`
    /// pairs joined with the `knowledge` table.
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<(f32, KnowledgeEntry)>> {
        let q_emb = embed_text(query, EMBED_DIM);
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT e.key, e.dim, e.vector, e.source_text, k.value, k.source, k.confidence,
                    k.created_at_ms, k.last_used_ms, k.use_count
             FROM knowledge_embeddings e
             LEFT JOIN knowledge k ON k.key = e.key",
        )?;
        let mut rows = stmt.query([])?;
        let mut scored: Vec<(f32, KnowledgeEntry)> = Vec::new();
        while let Some(row) = rows.next()? {
            let dim: i64 = row.get(1)?;
            let blob: Vec<u8> = row.get(2)?;
            if dim as usize != q_emb.dim || blob.len() != dim as usize * 4 {
                continue;
            }
            let values: Vec<f32> = blob
                .chunks_exact(4)
                .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                .collect();
            let emb = Embedding {
                dim: dim as usize,
                values,
            };
            let score = q_emb.cosine(&emb);
            if score <= 0.0 {
                continue;
            }
            // Bump use count.
            let key: String = row.get(0)?;
            let _ = conn.execute(
                "UPDATE knowledge SET use_count=use_count+1, last_used_ms=?1 WHERE key=?2",
                params![
                    time::OffsetDateTime::now_utc().unix_timestamp() * 1000,
                    &key
                ],
            );
            let entry = KnowledgeEntry {
                key,
                value: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
                source: row.get(5)?,
                confidence: row.get(6)?,
                created_at_ms: row.get::<_, i64>(7)? as u64,
                last_used_ms: row.get::<_, i64>(8)? as u64,
                use_count: row.get::<_, i64>(9)? as u64,
            };
            scored.push((score, entry));
        }
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        Ok(scored.into_iter().take(limit).collect())
    }

    /// Backfill embeddings for every knowledge entry that doesn't yet
    /// have one. Useful for first-time migration after v0.5 install.
    pub fn backfill(&self) -> Result<usize> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT k.key, k.value FROM knowledge k
             LEFT JOIN knowledge_embeddings e ON e.key = k.key
             WHERE e.key IS NULL",
        )?;
        let mut rows = stmt.query([])?;
        let mut to_embed: Vec<(String, String)> = Vec::new();
        while let Some(row) = rows.next()? {
            let key: String = row.get(0)?;
            let value: String = row.get(1)?;
            to_embed.push((key, value));
        }
        drop(rows);
        drop(stmt);
        // Drop the borrow so we can call upsert.
        drop(conn);
        let mut count = 0usize;
        for (k, v) in &to_embed {
            let text = format!("{k} {v}");
            self.upsert(k, &text)?;
            count += 1;
        }
        Ok(count)
    }

    /// Returns a histogram of buckets → number of entries with nonzero
    /// weight in that bucket. Mostly useful for diagnostics.
    pub fn stats(&self) -> Result<HashMap<usize, u32>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare("SELECT vector, dim FROM knowledge_embeddings")?;
        let mut rows = stmt.query([])?;
        let mut hist: HashMap<usize, u32> = HashMap::new();
        while let Some(row) = rows.next()? {
            let dim: i64 = row.get(1)?;
            let blob: Vec<u8> = row.get(0)?;
            for (i, chunk) in blob.chunks_exact(4).enumerate() {
                let f = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                if f.abs() > 0.01 {
                    *hist.entry(i).or_insert(0) += 1;
                }
            }
            let _ = dim; // dim not used in histogram
        }
        Ok(hist)
    }
}

/// Create the `knowledge_embeddings` table. Called from `MemoryStore::migrate`.
pub fn migrate(conn: &rusqlite::Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS knowledge_embeddings (
            key           TEXT PRIMARY KEY,
            dim           INTEGER NOT NULL,
            vector        BLOB NOT NULL,
            source_text   TEXT NOT NULL,
            updated_at_ms INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_knowledge_embeddings_key
            ON knowledge_embeddings(key);
        "#,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embed_text_returns_normalized_vector() {
        let e = embed_text("Hello world this is a test", EMBED_DIM);
        assert_eq!(e.dim, EMBED_DIM);
        let norm: f32 = e.values.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.01);
    }

    #[test]
    fn cosine_similarity_highest_for_identical_text() {
        let a = embed_text("schedule a meeting with Bob", EMBED_DIM);
        let b = embed_text("schedule a meeting with Bob", EMBED_DIM);
        let c = embed_text("the quick brown fox jumps over the lazy dog", EMBED_DIM);
        assert!(a.cosine(&b) > a.cosine(&c));
    }

    #[test]
    fn trigram_handles_typos() {
        // "calendar" vs "calender" should still match strongly because
        // 6 of 7 trigrams overlap (cal, ale, len, end, nda, dar).
        let a = embed_text("calendar", EMBED_DIM);
        let b = embed_text("calender", EMBED_DIM);
        let score = a.cosine(&b);
        assert!(
            score > 0.5,
            "expected typo-tolerant score > 0.5, got {score}"
        );
    }

    #[test]
    fn zeros_cosine_returns_zero() {
        let a = Embedding::zeros(8);
        let b = embed_text("anything", 8);
        assert_eq!(a.cosine(&b), 0.0);
    }

    #[test]
    fn dim_mismatch_returns_zero() {
        let a = embed_text("foo", 8);
        let b = embed_text("foo", 16);
        assert_eq!(a.cosine(&b), 0.0);
    }
}
