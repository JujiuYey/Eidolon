use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{de::DeserializeOwned, Serialize};

/// 本地 JSON 文件存储服务
pub struct LocalJsonStore {
    /// 数据目录路径
    data_dir: PathBuf,
}

impl LocalJsonStore {
    /// 创建新的存储实例
    ///
    /// # Arguments
    /// * `data_dir` - 数据存储目录路径
    ///
    /// # Returns
    /// 初始化后的存储实例
    pub fn new(data_dir: PathBuf) -> Result<Self, String> {
        // 确保目录存在
        fs::create_dir_all(&data_dir).map_err(|error| format!("创建数据目录失败: {error}"))?;

        Ok(Self { data_dir })
    }

    /// 获取数据目录路径
    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    /// 读取 JSON 文件
    ///
    /// # Type Parameters
    /// * `T` - 反序列化目标类型
    ///
    /// # Arguments
    /// * `filename` - 文件名（不含扩展名）
    ///
    /// # Returns
    /// 反序列化后的数据，文件不存在时返回 None
    pub fn read<T: DeserializeOwned + Default>(&self, filename: &str) -> Result<T, String> {
        let file_path = self.file_path(filename);

        if !file_path.exists() {
            // 文件不存在时返回默认值
            return Ok(T::default());
        }

        let content = fs::read_to_string(&file_path)
            .map_err(|error| format!("读取文件失败 ({}): {error}", file_path.display()))?;

        serde_json::from_str(&content).map_err(|error| format!("解析 JSON 失败: {error}"))
    }

    /// 写入 JSON 文件
    ///
    /// # Type Parameters
    /// * `T` - 序列化数据类型
    ///
    /// # Arguments
    /// * `filename` - 文件名（不含扩展名）
    /// * `data` - 要写入的数据
    pub fn write<T: Serialize>(&self, filename: &str, data: &T) -> Result<(), String> {
        let file_path = self.file_path(filename);

        let content = serde_json::to_string_pretty(data)
            .map_err(|error| format!("序列化 JSON 失败: {error}"))?;

        fs::write(&file_path, content)
            .map_err(|error| format!("写入文件失败 ({}): {error}", file_path.display()))
    }

    /// 生成完整的文件路径
    fn file_path(&self, filename: &str) -> PathBuf {
        self.data_dir.join(format!("{}.json", filename))
    }
}

/// 获取当前时间（UTC）
pub fn now() -> DateTime<Utc> {
    Utc::now()
}
