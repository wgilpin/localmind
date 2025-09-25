use crate::Result;

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub doc_id: i64,
    pub similarity: f32,
}

pub struct VectorStore {
    vectors: Vec<(i64, Vec<f32>)>, // (doc_id, vector)
}

impl VectorStore {
    pub fn new() -> Self {
        Self {
            vectors: Vec::new(),
        }
    }

    pub fn load_vectors(&mut self, vectors: Vec<(i64, Vec<f32>)>) -> Result<()> {
        self.vectors = vectors;
        Ok(())
    }

    pub fn add_vector(&mut self, doc_id: i64, vector: Vec<f32>) -> Result<()> {
        self.vectors.push((doc_id, vector));
        Ok(())
    }

    pub fn search(&self, query_vector: &[f32], limit: usize) -> Result<Vec<SearchResult>> {
        if query_vector.is_empty() {
            return Ok(vec![]);
        }

        let mut similarities: Vec<SearchResult> = Vec::new();

        for (doc_id, vector) in &self.vectors {
            if let Some(similarity) = cosine_similarity(query_vector, vector) {
                similarities.push(SearchResult {
                    doc_id: *doc_id,
                    similarity,
                });
            }
        }

        // Sort by similarity (highest first)
        similarities.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap_or(std::cmp::Ordering::Equal));

        // Take top results
        similarities.truncate(limit);

        Ok(similarities)
    }

    pub fn len(&self) -> usize {
        self.vectors.len()
    }

    pub fn is_empty(&self) -> bool {
        self.vectors.is_empty()
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> Option<f32> {
    if a.len() != b.len() || a.is_empty() {
        return None;
    }

    let mut dot_product = 0.0;
    let mut norm_a = 0.0;
    let mut norm_b = 0.0;

    for i in 0..a.len() {
        dot_product += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];
    }

    let norm_a = norm_a.sqrt();
    let norm_b = norm_b.sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return Some(0.0);
    }

    Some(dot_product / (norm_a * norm_b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity() {
        let vec1 = vec![1.0, 2.0, 3.0];
        let vec2 = vec![1.0, 2.0, 3.0];
        let similarity = cosine_similarity(&vec1, &vec2).unwrap();
        assert!((similarity - 1.0).abs() < 1e-6);

        let vec3 = vec![1.0, 0.0, 0.0];
        let vec4 = vec![0.0, 1.0, 0.0];
        let similarity = cosine_similarity(&vec3, &vec4).unwrap();
        assert!((similarity - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_vector_search() {
        let mut store = VectorStore::new();
        store.add_vector(1, vec![1.0, 0.0, 0.0]).unwrap();
        store.add_vector(2, vec![0.8, 0.6, 0.0]).unwrap();
        store.add_vector(3, vec![0.0, 1.0, 0.0]).unwrap();

        let query = vec![1.0, 0.0, 0.0];
        let results = store.search(&query, 2).unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].doc_id, 1);
        assert!(results[0].similarity > results[1].similarity);
    }
}