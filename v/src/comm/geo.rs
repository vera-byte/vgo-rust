use serde::{Deserialize, Serialize};
use thiserror::Error;

// 统一错误类型 / Unified error type
#[derive(Debug, Error)]
pub enum GeoError {
    #[error("HTTP错误: {0}")]
    Http(String),
    #[error("配置错误: {0}")]
    Config(String),
    #[error("接口返回错误: {0}")]
    Api(String),
}

// 省市区信息 / Province-City-District info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionInfo {
    pub ip: Option<String>,       // IP地址 / IP address
    pub province: Option<String>, // 省 / province
    pub city: Option<String>,     // 市 / city
    pub district: Option<String>, // 区/县 / district
    pub adcode: Option<String>,   // 行政区代码 / administrative code
}

// 获取公网IP（使用 3322 服务，返回纯文本）
// Fetch public IP (via 3322 service, plain text response)
pub async fn get_public_ip() -> Result<String, GeoError> {
    let resp = reqwest::Client::new()
        .get("https://ip.3322.net")
        .send()
        .await
        .map_err(|e| GeoError::Http(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(GeoError::Http(format!("status={}", resp.status())));
    }
    let body = resp
        .text()
        .await
        .map_err(|e| GeoError::Http(e.to_string()))?;
    let ip = body.trim();
    // 验证IP格式（IPv4/IPv6）
    // Validate IP format (IPv4/IPv6)
    if ip.parse::<std::net::IpAddr>().is_ok() {
        Ok(ip.to_string())
    } else {
        Err(GeoError::Http(format!("invalid ip text: {}", ip)))
    }
}

// 通过高德IP定位获取省市区信息（可选指定IP；若为空则自动获取公网IP）
// Get province/city/district via Amap IP API (optionally specify IP; defaults to public IP)
pub async fn get_region_by_ip(ip: Option<&str>) -> Result<RegionInfo, GeoError> {
    let cm = crate::comm::config::get_global_config_manager()
        .map_err(|e| GeoError::Config(e.to_string()))?;
    let mut key: String = cm.get_or("amap.key", "".to_string());
    if key.is_empty() {
        if let Ok(k) = std::env::var("V_AMAP_KEY") {
            key = k;
        }
    }
    if key.is_empty() {
        return Err(GeoError::Config(
            "amap.key 未配置 / AMAP key is missing".to_string(),
        ));
    }

    // 优先参数，其次环境/配置；不访问外网
    let ip_val = if let Some(v) = ip {
        v.to_string()
    } else {
        if let Ok(v) = std::env::var("V_AMAP_IP") {
            if !v.is_empty() {
                v
            } else {
                String::new()
            }
        } else {
            String::new()
        }
    };
    let ip_val = if ip_val.is_empty() {
        let cm_ip2: String = cm.get_or("server.public_ip", "".to_string());
        if !cm_ip2.is_empty() {
            cm_ip2
        } else {
            if let Ok(v) = std::env::var("V_PUBLIC_IP") {
                if !v.is_empty() {
                    v
                } else {
                    String::new()
                }
            } else {
                String::new()
            }
        }
    } else {
        ip_val
    };
    let ip_val = if ip_val.is_empty() {
        // 最后回退到外网服务（如可访问）
        // Finally fallback to external service (if reachable)
        match get_public_ip().await {
            Ok(v) => v,
            Err(_) => String::new(),
        }
    } else {
        ip_val
    };
    if ip_val.is_empty() {
        return Err(GeoError::Config(
            "ip 未提供，且未配置 V_AMAP_IP/V_PUBLIC_IP 或 server.public_ip，且无法外网探测"
                .to_string(),
        ));
    }

    // 调用高德 IP 定位 / Call Amap IP API
    // 高德可能返回字符串或数组（例如 city: []），使用多态解析
    // Amap may return string or array (e.g., city: []), use untagged parsing
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum TextOrArray {
        Text(String),
        Array(Vec<String>),
        Null,
    }

    #[derive(Deserialize)]
    struct AmapIpResp {
        status: String,
        info: Option<String>,
        province: Option<TextOrArray>,
        city: Option<TextOrArray>,
        adcode: Option<TextOrArray>,
    }
    let url = format!("https://restapi.amap.com/v3/ip?ip={}&key={}", ip_val, key);
    let resp = reqwest::Client::new()
        .get(&url)
        .send()
        .await
        .map_err(|e| GeoError::Http(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(GeoError::Http(format!("status={}", resp.status())));
    }
    let ip_data: AmapIpResp = resp
        .json()
        .await
        .map_err(|e| GeoError::Http(e.to_string()))?;
    if ip_data.status != "1" {
        return Err(GeoError::Api(
            ip_data.info.unwrap_or_else(|| "unknown".to_string()),
        ));
    }

    // 提取文本值（优先取第一个元素）
    // Extract text value (prefer first element)
    fn to_opt_string(v: Option<TextOrArray>) -> Option<String> {
        match v {
            Some(TextOrArray::Text(s)) => {
                if s.is_empty() {
                    None
                } else {
                    Some(s)
                }
            }
            Some(TextOrArray::Array(arr)) => arr.into_iter().find(|s| !s.is_empty()),
            _ => None,
        }
    }

    let province_str = to_opt_string(ip_data.province);
    let city_str = to_opt_string(ip_data.city);
    let adcode_str = to_opt_string(ip_data.adcode);

    let mut region = RegionInfo {
        ip: Some(ip_val),
        province: province_str,
        city: city_str,
        district: None,
        adcode: adcode_str,
    };

    // 若返回了 adcode，尝试解析区县名称 / If adcode present, try resolve district name
    if let Some(ad) = &region.adcode {
        #[derive(Deserialize)]
        struct DistrictItem {
            name: Option<String>,
            level: Option<String>,
        }
        #[derive(Deserialize)]
        struct DistrictResp {
            status: String,
            districts: Option<Vec<DistrictItem>>,
            info: Option<String>,
        }

        let url2 = format!(
            "https://restapi.amap.com/v3/config/district?keywords={}&subdistrict=0&key={}",
            ad, key
        );
        let resp2 = reqwest::Client::new()
            .get(&url2)
            .send()
            .await
            .map_err(|e| GeoError::Http(e.to_string()))?;
        if resp2.status().is_success() {
            let d: DistrictResp = resp2
                .json()
                .await
                .map_err(|e| GeoError::Http(e.to_string()))?;
            if d.status == "1" {
                if let Some(list) = d.districts {
                    if let Some(item) = list.get(0) {
                        if item.level.as_deref() == Some("district") {
                            region.district = item.name.clone();
                        }
                    }
                }
            }
        }
    }

    Ok(region)
}
