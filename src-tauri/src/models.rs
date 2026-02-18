use serde::{Deserialize, Serialize};

/// 分镜条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Storyboard {
    pub sequence_number: i64,
    pub mirror_id: String,
    pub shot_type: Option<String>,
    pub shot_size: Option<String>,
    pub duration: Option<f64>,
    pub dialogue: Option<String>,
    pub description: Option<String>,
    pub notes: Option<String>,
    pub image_prompt_zh: Option<String>,
    pub image_prompt_en: Option<String>,
    pub image_prompt_tail_zh: Option<String>,
    pub image_prompt_tail_en: Option<String>,
    pub video_prompt_zh: Option<String>,
    pub video_prompt_en: Option<String>,
    pub image_first_path: Option<String>,
    pub image_last_path: Option<String>,
    pub image_status: Option<String>,
}

/// 角色资产
/// 支持 AI 可能返回的多种字段名：image_prompt_zh/prompt_cn, image_prompt_en/prompt_en, notes/remarks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Character {
    pub name: String,
    pub description: Option<String>,
    #[serde(alias = "prompt_cn")]
    pub image_prompt_zh: Option<String>,
    #[serde(alias = "prompt_en")]
    pub image_prompt_en: Option<String>,
    #[serde(alias = "remarks")]
    pub notes: Option<String>,
}

/// 场景资产
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
    pub name: String,
    pub description: Option<String>,
    #[serde(alias = "prompt_cn")]
    pub image_prompt_zh: Option<String>,
    #[serde(alias = "prompt_en")]
    pub image_prompt_en: Option<String>,
    #[serde(alias = "remarks")]
    pub notes: Option<String>,
}

/// 道具资产
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prop {
    pub name: String,
    pub description: Option<String>,
    #[serde(alias = "prompt_cn")]
    pub image_prompt_zh: Option<String>,
    #[serde(alias = "prompt_en")]
    pub image_prompt_en: Option<String>,
    #[serde(alias = "remarks")]
    pub notes: Option<String>,
}

/// AI 消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: Option<i64>,
    pub role: String,
    pub content: String,
    pub timestamp: Option<i64>,
}

/// API 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    pub id: String,
    pub name: String,
    pub api_type: String, // text, image, video
    pub base_url: String,
    pub api_key: String,
    pub model: Option<String>,
    pub is_default: bool,
}

/// 项目元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMeta {
    pub name: String,
    pub path: String,
    pub created_at: i64,
    pub modified_at: i64,
    pub storyboard_count: i64,
    pub chat_count: i64,
}

/// AI 生成响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiGenerateResponse {
    pub storyboards: Vec<Storyboard>,
    pub characters: Vec<Character>,
    pub scenes: Vec<Scene>,
    pub props: Vec<Prop>,
}

/// 风格提示词
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StylePrompts {
    pub style_zh: String,
    pub style_en: String,
}

/// 项目风格配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectStyle {
    pub style_prompt: Option<String>,
    pub quality_prompt: Option<String>,
}

/// 全局配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalConfig {
    pub apis: Vec<ApiConfig>,
    pub base_folder: Option<String>,
    pub last_project: Option<String>,
}
