use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    pub engine_type: String,
    pub model_path: Option<String>,
    pub num_threads: u32,
    #[serde(default = "default_graph_opt_level")]
    pub graph_opt_level: String,
    #[serde(default = "default_max_seq_len")]
    pub max_seq_len: usize,
    #[serde(default = "default_pooling_strategy")]
    pub pooling_strategy: String,
    #[serde(default = "default_l2_normalize")]
    pub l2_normalize: bool,
}

fn default_graph_opt_level() -> String {
    "level3".to_string()
}

fn default_max_seq_len() -> usize {
    512
}

fn default_pooling_strategy() -> String {
    "mean".to_string()
}

fn default_l2_normalize() -> bool {
    true
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            engine_type: "builtin".to_string(),
            model_path: None,
            num_threads: num_cpus::get().max(1) as u32,
            graph_opt_level: default_graph_opt_level(),
            max_seq_len: default_max_seq_len(),
            pooling_strategy: default_pooling_strategy(),
            l2_normalize: default_l2_normalize(),
        }
    }
}
