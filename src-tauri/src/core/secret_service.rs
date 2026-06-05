use std::collections::HashMap;
use std::path::{Path, PathBuf};
use parking_lot::Mutex;

/// SecretService：API Key 安全存储
/// 在 Windows 上使用加密文件持久化存储（理想情况下应使用 Windows Credential Manager）
/// API Key 永远不写入日志
pub struct SecretService {
    secrets: Mutex<HashMap<String, String>>,
    storage_dir: PathBuf,
}

impl SecretService {
    pub fn new(storage_dir: &Path) -> Self {
        let storage_dir = storage_dir.to_path_buf();
        let mut secrets = HashMap::new();
        // 从磁盘加载已持久化的密钥
        let secrets_file = storage_dir.join("secrets.dat");
        if secrets_file.exists() {
            if let Ok(encrypted) = std::fs::read_to_string(&secrets_file) {
                if let Ok(decrypted) = Self::decrypt(&encrypted) {
                    if let Ok(loaded) = serde_json::from_str::<HashMap<String, String>>(&decrypted) {
                        secrets = loaded;
                        log::info!("已从磁盘加载 {} 个 API Key", secrets.len());
                    }
                }
            }
        }
        Self {
            secrets: Mutex::new(secrets),
            storage_dir,
        }
    }

    /// 安全存储 API Key（内存 + 加密磁盘持久化）
    pub fn store_api_key(&self, key_name: &str, api_key: &str) {
        {
            let mut secrets = self.secrets.lock();
            secrets.insert(key_name.to_string(), api_key.to_string());
        }
        self.persist_secrets();
        log::info!("API Key 已安全存储 (key_name: {})", key_name);
    }

    /// 获取 API Key
    pub fn get_api_key(&self, key_name: &str) -> Option<String> {
        let secrets = self.secrets.lock();
        secrets.get(key_name).cloned()
    }

    /// 删除 API Key
    pub fn remove_api_key(&self, key_name: &str) {
        {
            let mut secrets = self.secrets.lock();
            secrets.remove(key_name);
        }
        self.persist_secrets();
        log::info!("API Key 已删除 (key_name: {})", key_name);
    }

    /// 脱敏显示 API Key（仅显示前4后4位）
    pub fn mask_api_key(&self, key_name: &str) -> Option<String> {
        self.get_api_key(key_name).map(|key| {
            if key.len() <= 8 {
                "****".to_string()
            } else {
                format!("{}****{}", &key[..4], &key[key.len()-4..])
            }
        })
    }

    /// 检查 API Key 是否已设置
    pub fn has_api_key(&self, key_name: &str) -> bool {
        let secrets = self.secrets.lock();
        secrets.get(key_name).map(|s| !s.is_empty()).unwrap_or(false)
    }

    /// 检查任意 API Key 是否已设置（用于 pre-check）
    pub fn has_any_key(&self) -> bool {
        let secrets = self.secrets.lock();
        secrets.values().any(|v| !v.is_empty())
    }

    /// 持久化所有密钥到磁盘
    fn persist_secrets(&self) {
        let secrets = self.secrets.lock();
        if let Ok(json) = serde_json::to_string(&*secrets) {
            let encrypted = Self::encrypt(&json);
            let secrets_file = self.storage_dir.join("secrets.dat");
            if let Err(e) = std::fs::create_dir_all(&self.storage_dir) {
                log::error!("创建密钥存储目录失败: {}", e);
                return;
            }
            if let Err(e) = std::fs::write(&secrets_file, &encrypted) {
                log::error!("持久化密钥失败: {}", e);
            }
        }
    }

    /// 简单加密（XOR + Base64）
    fn encrypt(data: &str) -> String {
        let key = b"llmwiki_secret_key_2024_v1";
        let bytes: Vec<u8> = data.bytes()
            .enumerate()
            .map(|(i, b)| b ^ key[i % key.len()])
            .collect();
        use base64::Engine;
        base64::engine::general_purpose::STANDARD.encode(&bytes)
    }

    /// 简单解密
    fn decrypt(encrypted: &str) -> Result<String, String> {
        use base64::Engine;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(encrypted)
            .map_err(|e| format!("Base64解码失败: {}", e))?;
        let key = b"llmwiki_secret_key_2024_v1";
        let decrypted: Vec<u8> = bytes.iter()
            .enumerate()
            .map(|(i, b)| b ^ key[i % key.len()])
            .collect();
        String::from_utf8(decrypted)
            .map_err(|e| format!("UTF-8解码失败: {}", e))
    }
}
