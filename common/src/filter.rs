use serde::{Deserialize, Serialize};

/// 定义"Ani"的嵌套的查询参数结构
#[derive(Debug, Deserialize, Clone)]
pub struct AniFilter {
    pub title: Option<String>,
    pub platform: Option<String>,
}

/// 定义"News"的嵌套的查询参数结构
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewsFilter {
    pub news_from: Option<String>,
    pub news_date: Option<String>,
    pub extracted: Option<bool>,
}

/// 定义"Task"的嵌套的查询参数结构
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskFilter {
    pub name: Option<String>,
    pub arg: Option<String>,
    pub cmd: Option<String>,
    pub is_enabled: Option<bool>,
}
