#![allow(dead_code)]
use reqwest::Client;
use serde::{Serialize, Deserialize};
use std::env;
use std::collections::HashMap;
use tokio::sync::RwLock;
use std::sync::Arc;

/// OpenAI Embedding配置
#[derive(Debug, Clone)]
pub struct EmbeddingConfig {
    pub api_key: String,
    pub model: String,
    pub base_url: String,
    pub max_batch_size: usize,
    pub max_tokens_per_batch: usize,
}

impl EmbeddingConfig {
    pub fn from_env() -> Result<Self, String> {
        let api_key = env::var("OPENAI_API_KEY")
            .or_else(|_| env::var("EMBEDDING_API_KEY"))
            .map_err(|_| "OPENAI_API_KEY or EMBEDDING_API_KEY not set")?;
        
        let model = env::var("EMBEDDING_MODEL")
            .unwrap_or_else(|_| "text-embedding-3-small".to_string());
        
        let base_url = env::var("OPENAI_BASE_URL")
            .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
        
        let max_batch_size = env::var("EMBEDDING_MAX_BATCH_SIZE")
            .unwrap_or_else(|_| "100".to_string())
            .parse()
            .unwrap_or(100);
        
        Ok(Self { 
            api_key, 
            model, 
            base_url,
            max_batch_size,
            max_tokens_per_batch: 8000,  // OpenAI限制
        })
    }
}

/// Embedding请求
#[derive(Debug, Serialize)]
struct EmbeddingRequest {
    input: Vec<String>,
    model: String,
}

/// Embedding响应
#[derive(Debug, Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
}

#[derive(Debug, Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
    index: usize,
}

/// Embedding缓存
pub struct EmbeddingCache {
    cache: Arc<RwLock<HashMap<String, Vec<f32>>>>,
}

impl EmbeddingCache {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    pub async fn get(&self, text: &str) -> Option<Vec<f32>> {
        let cache = self.cache.read().await;
        cache.get(text).cloned()
    }
    
    pub async fn set(&self, text: String, embedding: Vec<f32>) {
        let mut cache = self.cache.write().await;
        cache.insert(text, embedding);
    }
    
    pub async fn get_batch(&self, texts: &[String]) -> (Vec<usize>, Vec<String>) {
        let cache = self.cache.read().await;
        let mut missing_indices = Vec::new();
        let mut missing_texts = Vec::new();
        
        for (i, text) in texts.iter().enumerate() {
            if !cache.contains_key(text) {
                missing_indices.push(i);
                missing_texts.push(text.clone());
            }
        }
        
        (missing_indices, missing_texts)
    }
}

/// 向量搜索服务（优化版：带缓存和批量限制）
pub struct VectorSearch {
    config: EmbeddingConfig,
    client: Client,
    cache: EmbeddingCache,
}

impl VectorSearch {
    pub fn new(config: EmbeddingConfig) -> Self {
        Self {
            config,
            client: Client::new(),
            cache: EmbeddingCache::new(),
        }
    }
    
    /// 生成单个文本的embedding（带缓存）
    pub async fn get_embedding(&self, text: &str) -> Result<Vec<f32>, String> {
        // 检查缓存
        if let Some(cached) = self.cache.get(text).await {
            return Ok(cached);
        }
        
        let embeddings = self.get_embeddings(vec![text.to_string()]).await?;
        let embedding = embeddings.into_iter().next().ok_or_else(|| "No embedding returned".to_string())?;
        
        // 缓存结果
        self.cache.set(text.to_string(), embedding.clone()).await;
        
        Ok(embedding)
    }
    
    /// 批量生成embeddings（带分批和缓存）
    pub async fn get_embeddings(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, String> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        
        // 检查缓存，找出需要请求的部分
        let (_missing_indices, missing_texts) = self.cache.get_batch(&texts).await;
        
        if missing_texts.is_empty() {
            // 全部命中缓存
            let cache = self.cache.cache.read().await;
            return Ok(texts.iter().map(|t| cache.get(t).cloned().unwrap_or_default()).collect());
        }
        
        // 分批请求
        let mut all_embeddings: HashMap<String, Vec<f32>> = HashMap::new();
        
        for chunk in missing_texts.chunks(self.config.max_batch_size) {
            let request = EmbeddingRequest {
                input: chunk.to_vec(),
                model: self.config.model.clone(),
            };
            
            let url = format!("{}/embeddings", self.config.base_url);
            
            let response = self.client
                .post(&url)
                .header("Authorization", format!("Bearer {}", self.config.api_key))
                .header("Content-Type", "application/json")
                .json(&request)
                .send()
                .await
                .map_err(|e| format!("Request failed: {}", e))?;
            
            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                return Err(format!("API error {}: {}", status, body));
            }
            
            let embedding_response: EmbeddingResponse = response.json().await
                .map_err(|e| format!("Failed to parse response: {}", e))?;
            
            // 存入缓存
            for (i, data) in embedding_response.data.iter().enumerate() {
                if i < chunk.len() {
                    let text = chunk[i].clone();
                    self.cache.set(text.clone(), data.embedding.clone()).await;
                    all_embeddings.insert(text, data.embedding.clone());
                }
            }
        }
        
        // 按原始顺序返回
        let cache = self.cache.cache.read().await;
        let result: Vec<Vec<f32>> = texts.iter()
            .map(|t| cache.get(t).cloned().unwrap_or_default())
            .collect();
        
        Ok(result)
    }
    
    /// 计算余弦相似度
    pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() || a.is_empty() {
            return 0.0;
        }
        
        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        
        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }
        
        dot_product / (norm_a * norm_b)
    }
    
    /// 在embedding列表中搜索最相似的（优化：使用heap）
    pub fn find_most_similar(
        query_embedding: &[f32],
        embeddings: &[(String, Vec<f32>)],
        top_k: usize,
    ) -> Vec<(String, f32)> {
        use std::collections::BinaryHeap;
        use std::cmp::Ordering;
        
        #[derive(PartialEq)]
        struct ScoredItem {
            id: String,
            score: f32,
        }
        
        impl Eq for ScoredItem {}
        
        impl PartialOrd for ScoredItem {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                self.score.partial_cmp(&other.score)
            }
        }
        
        impl Ord for ScoredItem {
            fn cmp(&self, other: &Self) -> Ordering {
                self.partial_cmp(other).unwrap_or(Ordering::Equal)
            }
        }
        
        let mut heap = BinaryHeap::with_capacity(top_k + 1);
        
        for (id, emb) in embeddings {
            let sim = Self::cosine_similarity(query_embedding, emb);
            
            if heap.len() < top_k {
                heap.push(ScoredItem { id: id.clone(), score: sim });
            } else if let Some(mut min) = heap.peek_mut() {
                if sim > min.score {
                    *min = ScoredItem { id: id.clone(), score: sim };
                }
            }
        }
        
        let mut result: Vec<(String, f32)> = heap.into_iter()
            .map(|item| (item.id, item.score))
            .collect();
        
        result.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        result
    }
    
    /// 导出缓存（用于持久化）
    pub async fn export_cache(&self) -> serde_json::Value {
        let cache = self.cache.cache.read().await;
        let cache_map: HashMap<String, Vec<f32>> = cache.clone();
        serde_json::json!({
            "cache": cache_map
        })
    }
    
    /// 导入缓存
    pub async fn import_cache(&self, data: &serde_json::Value) {
        if let Some(cache_data) = data.get("cache") {
            if let Ok(parsed) = serde_json::from_value::<HashMap<String, Vec<f32>>>(cache_data.clone()) {
                let mut cache = self.cache.cache.write().await;
                *cache = parsed;
            }
        }
    }
}

/// 文本分块器（用于长文本embedding）
pub struct TextChunker {
    pub chunk_size: usize,
    pub overlap: usize,
}

impl TextChunker {
    pub fn new(chunk_size: usize, overlap: usize) -> Self {
        Self { chunk_size, overlap }
    }
    
    /// 将文本分块
    pub fn chunk_text(&self, text: &str) -> Vec<String> {
        let chars: Vec<char> = text.chars().collect();
        let mut chunks = Vec::new();
        let mut start = 0;
        
        while start < chars.len() {
            let end = std::cmp::min(start + self.chunk_size, chars.len());
            let chunk: String = chars[start..end].iter().collect();
            chunks.push(chunk);
            
            if end >= chars.len() {
                break;
            }
            
            start = end - self.overlap;
        }
        
        chunks
    }
    
    /// 按段落分块
    pub fn chunk_by_paragraph(&self, text: &str) -> Vec<String> {
        let paragraphs: Vec<&str> = text.split("\n\n").collect();
        let mut chunks = Vec::new();
        let mut current_chunk = String::new();
        
        for paragraph in paragraphs {
            if current_chunk.len() + paragraph.len() > self.chunk_size && !current_chunk.is_empty() {
                chunks.push(current_chunk.trim().to_string());
                current_chunk = String::new();
            }
            
            if !current_chunk.is_empty() {
                current_chunk.push_str("\n\n");
            }
            current_chunk.push_str(paragraph);
        }
        
        if !current_chunk.is_empty() {
            chunks.push(current_chunk.trim().to_string());
        }
        
        chunks
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((VectorSearch::cosine_similarity(&a, &b) - 1.0).abs() < 0.001);
        
        let c = vec![0.0, 1.0, 0.0];
        assert!((VectorSearch::cosine_similarity(&a, &c) - 0.0).abs() < 0.001);
    }
    
    #[test]
    fn test_text_chunker() {
        let chunker = TextChunker::new(10, 2);
        let text = "Hello World! This is a test.";
        let chunks = chunker.chunk_text(text);
        assert!(chunks.len() > 0);
    }
}
