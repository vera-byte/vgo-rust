//! 通用插件安装器（.vp 包）/ Generic plugin installer (.vp packages)

use anyhow::{anyhow, Result};
use flate2::read::GzDecoder;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tar::Archive;
use tracing::{debug, info, warn};

/// 插件安装器 / Plugin installer
pub struct PluginInstaller {
    plugin_dir: PathBuf,
}

/// 简易插件信息结构 / Simple plugin info
#[derive(serde::Deserialize)]
pub struct PluginInfoLite {
    pub name: String,
    #[allow(dead_code)]
    pub version: Option<String>,
    #[allow(dead_code)]
    pub description: Option<String>,
}

impl PluginInstaller {
    /// 创建新的插件安装器 / Create new plugin installer
    pub fn new(plugin_dir: impl AsRef<Path>) -> Self {
        Self {
            plugin_dir: plugin_dir.as_ref().to_path_buf(),
        }
    }

    /// 初始化插件目录 / Initialize plugin directory
    pub fn init(&self) -> Result<()> {
        if !self.plugin_dir.exists() {
            fs::create_dir_all(&self.plugin_dir)?;
            info!("Created plugin directory: {:?}", self.plugin_dir);
        }
        Ok(())
    }

    /// 从 URL 下载并安装插件 / Download and install plugin from URL
    pub async fn install_from_url(&self, url: &str) -> Result<String> {
        info!("Installing plugin from: {}", url);

        // 处理 file:// 协议 / Handle file:// protocol
        if url.starts_with("file://") {
            let file_path = url.strip_prefix("file://").unwrap();
            return self.install_from_file(file_path);
        }

        // 替换变量 / Replace variables
        let url = self.replace_variables(url)?;

        // 下载文件 / Download file
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()?;
        let response = client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Failed to download plugin: HTTP {}",
                response.status()
            ));
        }

        // 获取文件名 / Get filename
        let filename = url
            .split('/')
            .next_back()
            .ok_or_else(|| anyhow!("Invalid URL: no filename"))?;

        let temp_file = self.plugin_dir.join(format!("{}.tmp", filename));

        // 保存到临时文件 / Save to temporary file
        let mut file = fs::File::create(&temp_file)?;
        let bytes = response.bytes().await?;
        file.write_all(&bytes)?;
        file.sync_all()?;

        debug!("Downloaded plugin to: {:?}", temp_file);

        // 安装插件 / Install plugin
        let plugin_name = self.install_from_file(&temp_file)?;

        // 删除临时文件 / Remove temporary file
        let _ = fs::remove_file(&temp_file);

        info!("Plugin installed: {}", plugin_name);
        Ok(plugin_name)
    }

    /// 从本地文件安装插件 / Install plugin from local file
    pub fn install_from_file(&self, file_path: impl AsRef<Path>) -> Result<String> {
        let file_path = file_path.as_ref();

        if !file_path.exists() {
            return Err(anyhow!("Plugin file not found: {:?}", file_path));
        }

        // 检查文件扩展名（.vp）/ Check file extension (.vp)
        let ext = file_path.extension().and_then(|s| s.to_str()).unwrap_or("");
        if ext != "vp" {
            return Err(anyhow!("Invalid plugin file extension: {}", ext));
        }

        // 读取文件 / Read file
        let mut file = fs::File::open(file_path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;

        // 尝试解压 tar.gz / Try to extract tar.gz
        let plugin_name = if buffer.starts_with(&[0x1f, 0x8b]) {
            // gzip 格式 / gzip format
            self.extract_tar_gz(&buffer)?
        } else {
            return Err(anyhow!(
                "Unsupported plugin format, only tar.gz is supported"
            ));
        };

        Ok(plugin_name)
    }

    /// 解压 tar.gz 文件 / Extract tar.gz file
    fn extract_tar_gz(&self, data: &[u8]) -> Result<String> {
        let tar_gz = GzDecoder::new(data);
        let mut archive = Archive::new(tar_gz);

        // 创建临时解压目录 / Create temporary extraction directory
        let temp_dir = self.plugin_dir.join("temp_extract");
        if temp_dir.exists() {
            fs::remove_dir_all(&temp_dir)?;
        }
        fs::create_dir_all(&temp_dir)?;

        // 解压所有文件 / Extract all files
        archive.unpack(&temp_dir)?;

        // 查找插件信息文件 / Find plugin info file
        let plugin_name = self.find_plugin_name(&temp_dir)?;
        let plugin_dir = self.plugin_dir.join(&plugin_name);

        // 如果已存在，先删除 / Remove if exists
        if plugin_dir.exists() {
            warn!(
                "Plugin {} already exists, removing old version",
                plugin_name
            );
            fs::remove_dir_all(&plugin_dir)?;
        }

        // 如果解压目录下存在同名子目录，则移动子目录，否则移动整个解压目录
        let candidate_subdir = temp_dir.join(&plugin_name);
        if candidate_subdir.exists() && candidate_subdir.is_dir() {
            fs::rename(&candidate_subdir, &plugin_dir)?;
            let _ = fs::remove_dir_all(&temp_dir);
        } else {
            fs::rename(&temp_dir, &plugin_dir)?;
        }

        info!("Plugin extracted to: {:?}", plugin_dir);
        Ok(plugin_name)
    }

    /// 查找插件名称 / Find plugin name
    fn find_plugin_name(&self, dir: &Path) -> Result<String> {
        let entries: Vec<_> = fs::read_dir(dir)?.collect::<Result<_, _>>()?;

        // 先检查根目录下的配置文件 / Check config files at root
        for entry in &entries {
            let path = entry.path();
            if path.is_file() {
                if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                    if Self::is_plugin_config(name) {
                        let content = fs::read_to_string(&path)?;
                        if let Ok(plugin_info) = self.parse_plugin_info(&content) {
                            return Ok(plugin_info.name);
                        }
                    }
                }
            }
        }

        // 再检查子目录（tar 里包含插件根目录时）/ Check sub-directories for config
        for entry in &entries {
            let path = entry.path();
            if path.is_dir() {
                for cfg_name in ["plugin.json", "plugin.yaml", "plugin.yml"] {
                    let candidate = path.join(cfg_name);
                    if candidate.exists() {
                        let content = fs::read_to_string(&candidate)?;
                        if let Ok(plugin_info) = self.parse_plugin_info(&content) {
                            return Ok(plugin_info.name);
                        }
                    }
                }
            }
        }

        // 如果只有一个子目录，则使用子目录名 / Use single sub-directory name as fallback
        let dir_entries: Vec<_> = entries.iter().filter(|e| e.path().is_dir()).collect();
        if dir_entries.len() == 1 {
            if let Some(name) = dir_entries[0].file_name().to_str() {
                return Ok(name.to_string());
            }
        }

        // 最后回退到当前目录名 / Finally fallback to current directory name
        let dir_name = dir
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow!("Cannot determine plugin name"))?;

        Ok(dir_name.to_string())
    }

    #[inline]
    fn is_plugin_config(name: &str) -> bool {
        matches!(name, "plugin.json" | "plugin.yaml" | "plugin.yml")
    }

    /// 解析插件信息 / Parse plugin info
    fn parse_plugin_info(&self, content: &str) -> Result<PluginInfoLite> {
        if let Ok(info) = serde_json::from_str::<PluginInfoLite>(content) {
            return Ok(info);
        }
        Err(anyhow!("Failed to parse plugin info"))
    }

    /// 替换 URL 中的变量 / Replace variables in URL
    fn replace_variables(&self, url: &str) -> Result<String> {
        let os = match std::env::consts::OS {
            "macos" => "darwin",
            "linux" => "linux",
            "windows" => "windows",
            _ => return Err(anyhow!("Unsupported OS: {}", std::env::consts::OS)),
        };
        let arch = match std::env::consts::ARCH {
            "x86_64" => "amd64",
            "aarch64" => "arm64",
            _ => return Err(anyhow!(
                "Unsupported architecture: {}",
                std::env::consts::ARCH
            )),
        };
        Ok(url.replace("${os}", os).replace("${arch}", arch))
    }

    /// 列出已安装的插件 / List installed plugins
    pub fn list_installed(&self) -> Result<Vec<String>> {
        if !self.plugin_dir.exists() {
            return Ok(Vec::new());
        }
        let mut plugins = Vec::new();
        let entries = fs::read_dir(&self.plugin_dir)?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                    if name != "temp_extract" && name != "sockets" {
                        plugins.push(name.to_string());
                    }
                }
            }
        }
        Ok(plugins)
    }
}
