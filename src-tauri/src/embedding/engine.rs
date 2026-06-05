use std::path::Path;
use std::sync::Arc;

use ort::session::builder::GraphOptimizationLevel;
use ort::session::Session;
use ort::value::Value;
use parking_lot::Mutex;
use tokenizers::Tokenizer;

pub struct EmbeddingEngine {
    session: Mutex<Session>,
    tokenizer: Arc<Tokenizer>,
    model_dim: usize,
    pub num_threads: u32,
    pub max_seq_len: usize,
    pub pooling_strategy: String,
    pub l2_normalize: bool,
}

impl EmbeddingEngine {
    pub fn new(
        model_path: &Path,
        tokenizer_path: &Path,
        num_threads: u32,
        graph_opt_level: &str,
        max_seq_len: usize,
        pooling_strategy: &str,
        l2_normalize: bool,
    ) -> Result<Self, String> {
        // 绝对路径解析 + 存在性检查，防止断网时卡死
        let model_abs = std::path::absolute(model_path)
            .map_err(|e| format!("解析模型绝对路径失败: {}", e))?;
        if !model_abs.exists() {
            return Err(format!(
                "[路径解析失败] 在本地磁盘未找到 ONNX 模型: {:?}，请检查设置页路径。",
                model_abs
            ));
        }
        let tokenizer_abs = std::path::absolute(tokenizer_path)
            .map_err(|e| format!("解析 tokenizer 绝对路径失败: {}", e))?;
        if !tokenizer_abs.exists() {
            return Err(format!(
                "[路径解析失败] 在本地磁盘未找到 Tokenizer: {:?}",
                tokenizer_abs
            ));
        }

        let tokenizer = Tokenizer::from_file(&tokenizer_abs)
            .map_err(|e| format!("加载 tokenizer 失败: {}", e))?;

        let opt_level = match graph_opt_level {
            "level1" => GraphOptimizationLevel::Level1,
            "level2" => GraphOptimizationLevel::Level2,
            _ => GraphOptimizationLevel::Level3,
        };

        let session = Session::builder()
            .map_err(|e| format!("创建 session builder 失败: {}", e))?
            .with_optimization_level(opt_level)
            .map_err(|e| format!("设置优化级别失败: {}", e))?
            .with_intra_threads(num_threads as usize)
            .map_err(|e| format!("设置线程数失败: {}", e))?
            .commit_from_file(&model_abs)
            .map_err(|e| format!("加载 ONNX 模型失败: {}", e))?;

        // bge-small-zh-v1.5 outputs 512-dimensional embeddings
        let model_dim: usize = 512;

        log::info!(
            "[EmbeddingEngine] 模型加载完成, dim={}, threads={}, opt={}, seq_len={}, pooling={}, l2={}",
            model_dim, num_threads, graph_opt_level, max_seq_len, pooling_strategy, l2_normalize
        );

        Ok(Self {
            session: Mutex::new(session),
            tokenizer: Arc::new(tokenizer),
            model_dim,
            num_threads,
            max_seq_len,
            pooling_strategy: pooling_strategy.to_string(),
            l2_normalize,
        })
    }

    pub fn model_dim(&self) -> usize {
        self.model_dim
    }

    pub fn embed(&self, text: &str) -> Result<Vec<f32>, String> {
        let (input_ids, attention_mask) = self.tokenize(text)?;
        let embedding = self.run_inference(&input_ids, &attention_mask)?;
        Ok(embedding)
    }

    pub fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, String> {
        texts.iter().map(|t| self.embed(t)).collect()
    }

    fn tokenize(&self, text: &str) -> Result<(Vec<i64>, Vec<i64>), String> {
        let encoding = self
            .tokenizer
            .encode(text, true)
            .map_err(|e| format!("分词失败: {}", e))?;

        let ids: Vec<i64> = encoding.get_ids().iter().map(|&id| id as i64).collect();
        let mask: Vec<i64> = encoding
            .get_attention_mask()
            .iter()
            .map(|&m| m as i64)
            .collect();

        let len = ids.len().min(self.max_seq_len);
        Ok((ids[..len].to_vec(), mask[..len].to_vec()))
    }

    fn run_inference(
        &self,
        input_ids: &[i64],
        attention_mask: &[i64],
    ) -> Result<Vec<f32>, String> {
        let seq_len = input_ids.len();

        let shape: [usize; 2] = [1, seq_len];

        let ids_value = Value::from_array((shape, input_ids.to_vec()))
            .map_err(|e| format!("创建 input_ids tensor 失败: {}", e))?;
        let mask_value = Value::from_array((shape, attention_mask.to_vec()))
            .map_err(|e| format!("创建 attention_mask tensor 失败: {}", e))?;

        let mut session = self.session.lock();

        let outputs = session
            .run(ort::inputs![
                "input_ids" => ids_value,
                "attention_mask" => mask_value,
            ])
            .map_err(|e| format!("模型推理失败: {}", e))?;

        let (_shape, data): (&ort::value::Shape, &[f32]) = outputs["last_hidden_state"]
            .try_extract_tensor()
            .map_err(|e| format!("提取 last_hidden_state 失败: {}", e))?;

        let dim = self.model_dim;
        let mut embedding = vec![0.0f32; dim];
        let eps = 1e-12f32;

        match self.pooling_strategy.as_str() {
            "cls" => {
                // Use [CLS] token (first token) embedding
                for d in 0..dim {
                    embedding[d] = data[d];
                }
            }
            _ => {
                // Mean pooling over sequence dimension, weighted by attention mask
                let mut mask_sum: f32 = 0.0;
                for t in 0..seq_len {
                    let m = attention_mask[t] as f32;
                    if m > 0.0 {
                        mask_sum += m;
                        let offset = t * dim;
                        for d in 0..dim {
                            embedding[d] += data[offset + d] * m;
                        }
                    }
                }
                for d in 0..dim {
                    embedding[d] /= mask_sum.max(eps);
                }
            }
        }

        if self.l2_normalize {
            let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
            if norm > eps {
                for v in &mut embedding {
                    *v /= norm;
                }
            }
        }

        Ok(embedding)
    }
}
