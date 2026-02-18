use crate::db::{ProjectDatabase, get_config_dir, get_config_path};
use crate::models::*;
use std::fs;
use std::path::PathBuf;
use std::io::Read;
use serde_json::json;
use rfd::FileDialog;

/// 获取全局配置
#[tauri::command]
pub fn get_global_config() -> Result<GlobalConfig, String> {
    let config_path = get_config_path();
    if config_path.exists() {
        let content = fs::read_to_string(&config_path)
            .map_err(|e| format!("读取配置文件失败: {}", e))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("解析配置文件失败: {}", e))
    } else {
        Ok(GlobalConfig {
            apis: Vec::new(),
            base_folder: None,
            last_project: None,
        })
    }
}

/// 保存全局配置
#[tauri::command]
pub fn save_global_config(config: GlobalConfig) -> Result<(), String> {
    let config_dir = get_config_dir();
    fs::create_dir_all(&config_dir)
        .map_err(|e| format!("创建配置目录失败: {}", e))?;

    let config_path = get_config_path();
    let content = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("序列化配置失败: {}", e))?;

    fs::write(&config_path, content)
        .map_err(|e| format!("写入配置文件失败: {}", e))
}

/// 选择文件夹
#[tauri::command]
pub fn select_folder() -> Result<Option<String>, String> {
    let folder_path = FileDialog::new()
        .pick_folder();
    Ok(folder_path.map(|p| p.to_string_lossy().to_string()))
}

/// 检查是否是有效的项目目录
#[tauri::command]
pub fn is_valid_project(folder_path: String) -> Result<bool, String> {
    let path = PathBuf::from(&folder_path);
    let db_path = path.join(".storyboard").join("project.db");
    Ok(db_path.exists())
}

/// 创建新项目
#[tauri::command]
pub fn create_project(folder_path: String, project_name: String) -> Result<String, String> {
    let base_path = PathBuf::from(&folder_path);
    let project_path = base_path.join(&project_name);

    // 检查项目是否已存在
    if project_path.exists() {
        return Err("项目目录已存在".to_string());
    }

    // 创建项目目录
    fs::create_dir_all(&project_path)
        .map_err(|e| format!("创建项目目录失败: {}", e))?;

    // 创建 .storyboard 子目录
    let storyboard_dir = project_path.join(".storyboard");
    fs::create_dir_all(&storyboard_dir)
        .map_err(|e| format!("创建 .storyboard 目录失败: {}", e))?;

    // 创建数据库
    let _db = ProjectDatabase::open(&project_path)
        .map_err(|e| format!("创建数据库失败: {}", e))?;

    Ok(project_path.to_string_lossy().to_string())
}

/// 打开项目
#[tauri::command]
pub fn open_project(folder_path: String) -> Result<ProjectMeta, String> {
    let path = PathBuf::from(&folder_path);

    // 验证项目存在
    let db_path = path.join(".storyboard").join("project.db");
    if !db_path.exists() {
        return Err("不是有效的项目目录".to_string());
    }

    // 打开数据库获取信息
    let db = ProjectDatabase::open(&path)
        .map_err(|e| format!("打开数据库失败: {}", e))?;

    // 获取分镜数量
    let storyboard_count: i64 = db.conn().query_row(
        "SELECT COUNT(*) FROM storyboards",
        [],
        |row| row.get(0),
    ).unwrap_or(0);

    // 获取对话数量
    let chat_count: i64 = db.conn().query_row(
        "SELECT COUNT(*) FROM chat_history",
        [],
        |row| row.get(0),
    ).unwrap_or(0);

    // 获取创建时间和修改时间
    let metadata = fs::metadata(&path)
        .map_err(|e| format!("读取项目元数据失败: {}", e))?;
    let created_at = metadata.created()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let modified_at = metadata.modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let project_name = path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown")
        .to_string();

    Ok(ProjectMeta {
        name: project_name,
        path: folder_path,
        created_at,
        modified_at,
        storyboard_count,
        chat_count,
    })
}

/// 列出所有项目
#[tauri::command]
pub fn list_projects() -> Result<Vec<ProjectMeta>, String> {
    let config = get_global_config()?;
    let base_folder = config.base_folder
        .unwrap_or_else(|| dirs::home_dir()
            .unwrap()
            .join("Documents")
            .join("StoryboardProjects")
            .to_string_lossy()
            .to_string());

    let base_path = PathBuf::from(&base_folder);
    if !base_path.exists() {
        return Ok(Vec::new());
    }

    let mut projects = Vec::new();

    let entries = fs::read_dir(&base_path)
        .map_err(|e| format!("读取项目目录失败: {}", e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("读取目录项失败: {}", e))?;
        let path = entry.path();

        if path.is_dir() {
            let db_path = path.join(".storyboard").join("project.db");
            if db_path.exists() {
                if let Ok(project) = open_project(path.to_string_lossy().to_string()) {
                    projects.push(project);
                }
            }
        }
    }

    // 按修改时间排序
    projects.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));

    Ok(projects)
}

/// 检查项目名是否存在
#[tauri::command]
pub fn check_project_name_exists(project_name: String, base_folder: String) -> Result<bool, String> {
    let base_path = PathBuf::from(&base_folder);
    let project_path = base_path.join(&project_name);
    Ok(project_path.exists())
}

/// 更新项目名称
#[tauri::command]
pub fn update_project_name(folder_path: String, new_name: String) -> Result<(), String> {
    let old_path = PathBuf::from(&folder_path);
    let parent = old_path.parent()
        .ok_or("无法获取父目录")?;
    let new_path = parent.join(&new_name);

    if new_path.exists() {
        return Err("目标名称已存在".to_string());
    }

    fs::rename(&old_path, &new_path)
        .map_err(|e| format!("重命名失败: {}", e))
}

/// 保存生成的数据
#[tauri::command]
pub fn save_generated_data(
    folder_path: String,
    storyboards: Vec<Storyboard>,
    characters: Vec<Character>,
    scenes: Vec<Scene>,
    props: Vec<Prop>,
) -> Result<(), String> {
    let path = PathBuf::from(&folder_path);
    let db = ProjectDatabase::open(&path)
        .map_err(|e| format!("打开数据库失败: {}", e))?;

    eprintln!("=== 保存数据 ===");
    eprintln!("分镜数量: {}", storyboards.len());
    eprintln!("角色数量: {}", characters.len());
    eprintln!("场景数量: {}", scenes.len());
    eprintln!("道具数量: {}", props.len());

    // 保存分镜
    for storyboard in storyboards {
        db.conn().execute(
            "INSERT OR REPLACE INTO storyboards (
                mirror_id, sequence_number, shot_type, shot_size, duration,
                dialogue, description, notes,
                image_prompt_zh, image_prompt_en,
                image_prompt_tail_zh, image_prompt_tail_en,
                video_prompt_zh, video_prompt_en
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            [
                &storyboard.mirror_id,
                &storyboard.sequence_number.to_string(),
                &storyboard.shot_type.unwrap_or_default(),
                &storyboard.shot_size.unwrap_or_default(),
                &storyboard.duration.map(|d| d.to_string()).unwrap_or_default(),
                &storyboard.dialogue.unwrap_or_default(),
                &storyboard.description.unwrap_or_default(),
                &storyboard.notes.unwrap_or_default(),
                &storyboard.image_prompt_zh.unwrap_or_default(),
                &storyboard.image_prompt_en.unwrap_or_default(),
                &storyboard.image_prompt_tail_zh.unwrap_or_default(),
                &storyboard.image_prompt_tail_en.unwrap_or_default(),
                &storyboard.video_prompt_zh.unwrap_or_default(),
                &storyboard.video_prompt_en.unwrap_or_default(),
            ],
        ).map_err(|e| format!("保存分镜失败: {}", e))?;
    }

    // 保存角色
    eprintln!("开始保存 {} 个角色...", characters.len());
    for character in characters {
        eprintln!("  保存角色: {}", character.name);
        db.conn().execute(
            "INSERT OR REPLACE INTO characters (name, description, image_prompt_zh, image_prompt_en, notes)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            [
                &character.name,
                &character.description.unwrap_or_default(),
                &character.image_prompt_zh.unwrap_or_default(),
                &character.image_prompt_en.unwrap_or_default(),
                &character.notes.unwrap_or_default(),
            ],
        ).map_err(|e| format!("保存角色失败: {}", e))?;
    }

    // 保存场景
    eprintln!("开始保存 {} 个场景...", scenes.len());
    for scene in scenes {
        eprintln!("  保存场景: {}", scene.name);
        db.conn().execute(
            "INSERT OR REPLACE INTO scenes (name, description, image_prompt_zh, image_prompt_en, notes)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            [
                &scene.name,
                &scene.description.unwrap_or_default(),
                &scene.image_prompt_zh.unwrap_or_default(),
                &scene.image_prompt_en.unwrap_or_default(),
                &scene.notes.unwrap_or_default(),
            ],
        ).map_err(|e| format!("保存场景失败: {}", e))?;
    }

    // 保存道具
    eprintln!("开始保存 {} 个道具...", props.len());
    for prop in props {
        eprintln!("  保存道具: {}", prop.name);
        db.conn().execute(
            "INSERT OR REPLACE INTO props (name, description, image_prompt_zh, image_prompt_en, notes)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            [
                &prop.name,
                &prop.description.unwrap_or_default(),
                &prop.image_prompt_zh.unwrap_or_default(),
                &prop.image_prompt_en.unwrap_or_default(),
                &prop.notes.unwrap_or_default(),
            ],
        ).map_err(|e| format!("保存道具失败: {}", e))?;
    }

    Ok(())
}

/// 获取分镜列表
#[tauri::command]
pub fn get_storyboards(folder_path: String) -> Result<Vec<Storyboard>, String> {
    let path = PathBuf::from(&folder_path);
    let db = ProjectDatabase::open(&path)
        .map_err(|e| format!("打开数据库失败: {}", e))?;

    let mut stmt = db.conn().prepare(
        "SELECT sequence_number, mirror_id, shot_type, shot_size, duration,
                dialogue, description, notes,
                image_prompt_zh, image_prompt_en,
                image_prompt_tail_zh, image_prompt_tail_en,
                video_prompt_zh, video_prompt_en,
                image_first_path, image_last_path, image_status
         FROM storyboards ORDER BY sequence_number"
    ).map_err(|e| format!("查询分镜失败: {}", e))?;

    let storyboards = stmt.query_map([], |row| {
        Ok(Storyboard {
            sequence_number: row.get(0)?,
            mirror_id: row.get(1)?,
            shot_type: Some(row.get(2)?),
            shot_size: Some(row.get(3)?),
            duration: Some(row.get(4)?),
            dialogue: Some(row.get(5)?),
            description: Some(row.get(6)?),
            notes: Some(row.get(7)?),
            image_prompt_zh: Some(row.get(8)?),
            image_prompt_en: Some(row.get(9)?),
            image_prompt_tail_zh: Some(row.get(10)?),
            image_prompt_tail_en: Some(row.get(11)?),
            video_prompt_zh: Some(row.get(12)?),
            video_prompt_en: Some(row.get(13)?),
            image_first_path: row.get::<_, Option<String>>(14)?,
            image_last_path: row.get::<_, Option<String>>(15)?,
            image_status: row.get::<_, Option<String>>(16)?,
        })
    }).map_err(|e| format!("解析分镜失败: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("收集分镜失败: {}", e))?;

    Ok(storyboards)
}

/// 获取角色列表
#[tauri::command]
pub fn get_characters(folder_path: String) -> Result<Vec<Character>, String> {
    let path = PathBuf::from(&folder_path);
    let db = ProjectDatabase::open(&path)
        .map_err(|e| format!("打开数据库失败: {}", e))?;

    let mut stmt = db.conn().prepare(
        "SELECT name, description, image_prompt_zh, image_prompt_en, notes FROM characters"
    ).map_err(|e| format!("查询角色失败: {}", e))?;

    let characters = stmt.query_map([], |row| {
        Ok(Character {
            name: row.get(0)?,
            description: Some(row.get(1)?),
            image_prompt_zh: Some(row.get(2)?),
            image_prompt_en: Some(row.get(3)?),
            notes: Some(row.get(4)?),
        })
    }).map_err(|e| format!("解析角色失败: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("收集角色失败: {}", e))?;

    eprintln!("读取到 {} 个角色", characters.len());
    Ok(characters)
}

/// 获取场景列表
#[tauri::command]
pub fn get_scenes(folder_path: String) -> Result<Vec<Scene>, String> {
    let path = PathBuf::from(&folder_path);
    let db = ProjectDatabase::open(&path)
        .map_err(|e| format!("打开数据库失败: {}", e))?;

    let mut stmt = db.conn().prepare(
        "SELECT name, description, image_prompt_zh, image_prompt_en, notes FROM scenes"
    ).map_err(|e| format!("查询场景失败: {}", e))?;

    let scenes = stmt.query_map([], |row| {
        Ok(Scene {
            name: row.get(0)?,
            description: Some(row.get(1)?),
            image_prompt_zh: Some(row.get(2)?),
            image_prompt_en: Some(row.get(3)?),
            notes: Some(row.get(4)?),
        })
    }).map_err(|e| format!("解析场景失败: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("收集场景失败: {}", e))?;

    eprintln!("读取到 {} 个场景", scenes.len());
    Ok(scenes)
}

/// 获取道具列表
#[tauri::command]
pub fn get_props(folder_path: String) -> Result<Vec<Prop>, String> {
    let path = PathBuf::from(&folder_path);
    let db = ProjectDatabase::open(&path)
        .map_err(|e| format!("打开数据库失败: {}", e))?;

    let mut stmt = db.conn().prepare(
        "SELECT name, description, image_prompt_zh, image_prompt_en, notes FROM props"
    ).map_err(|e| format!("查询道具失败: {}", e))?;

    let props = stmt.query_map([], |row| {
        Ok(Prop {
            name: row.get(0)?,
            description: Some(row.get(1)?),
            image_prompt_zh: Some(row.get(2)?),
            image_prompt_en: Some(row.get(3)?),
            notes: Some(row.get(4)?),
        })
    }).map_err(|e| format!("解析道具失败: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("收集道具失败: {}", e))?;

    eprintln!("读取到 {} 个道具", props.len());
    Ok(props)
}

/// 保存聊天消息
#[tauri::command]
pub fn save_chat_message(folder_path: String, role: String, content: String) -> Result<(), String> {
    let path = PathBuf::from(&folder_path);
    let db = ProjectDatabase::open(&path)
        .map_err(|e| format!("打开数据库失败: {}", e))?;

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| format!("获取时间戳失败: {}", e))?
        .as_secs() as i64;

    db.conn().execute(
        "INSERT INTO chat_history (role, content, timestamp) VALUES (?1, ?2, ?3)",
        [&role, &content, &timestamp.to_string()],
    ).map_err(|e| format!("保存聊天消息失败: {}", e))?;

    Ok(())
}

/// 获取聊天历史
#[tauri::command]
pub fn get_chat_history(folder_path: String, limit: Option<i64>) -> Result<Vec<ChatMessage>, String> {
    let path = PathBuf::from(&folder_path);
    let db = ProjectDatabase::open(&path)
        .map_err(|e| format!("打开数据库失败: {}", e))?;

    let limit = limit.unwrap_or(20);
    let mut stmt = db.conn().prepare(
        &format!("SELECT id, role, content, timestamp FROM chat_history ORDER BY id DESC LIMIT {}", limit)
    ).map_err(|e| format!("查询聊天历史失败: {}", e))?;

    let messages: Vec<ChatMessage> = stmt.query_map([], |row| {
        Ok(ChatMessage {
            id: Some(row.get(0)?),
            role: row.get(1)?,
            content: row.get(2)?,
            timestamp: Some(row.get(3)?),
        })
    }).map_err(|e| format!("解析聊天历史失败: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("收集聊天历史失败: {}", e))?;

    // 反转顺序（最新的在最后）
    let messages: Vec<ChatMessage> = messages.into_iter().rev().collect();

    Ok(messages)
}

/// 调用 AI API
#[tauri::command]
pub fn call_ai_api(
    api_config: ApiConfig,
    message: String,
    chat_history: Option<Vec<ChatMessage>>,
) -> Result<String, String> {
    call_ai_api_with_custom_system(api_config, message, chat_history, None)
}

/// 调用 AI API - 支持自定义系统提示词版本
#[tauri::command]
pub fn call_ai_api_with_custom_system(
    api_config: ApiConfig,
    message: String,
    chat_history: Option<Vec<ChatMessage>>,
    custom_system_prompt: Option<String>,
) -> Result<String, String> {
    let base_url = api_config.base_url.trim_end_matches('/');
    let api_key = &api_config.api_key;
    let model = api_config.model.as_deref().unwrap_or("gpt-3.5-turbo");

    let url = format!("{}/chat/completions", base_url);

    let mut messages = Vec::new();

    // 使用自定义系统提示词（如果提供）
    if let Some(custom) = custom_system_prompt {
        messages.push(json!({
            "role": "system",
            "content": &custom
        }));
    } else {
        // 默认系统提示词
        messages.push(json!({
            "role": "system",
            "content": r#"你是一位拥有10年影视动画经验的资深职业分镜师，擅长镜头语言、叙事节奏、画面构图、运镜设计、剪辑逻辑。你的任务是把用户提供的剧本/文案/情节，严格转换成标准分镜脚本，遵循电影语言规范，不抒情、不文艺化、不脑补无关剧情，只做专业、可落地、可拍摄的分镜设计。

【分镜设计原则】
1. 精简原则：控制分镜数量！25秒内容一般不超过6-8个分镜，避免过度拆分
2. 遵循"一镜一意"原则，但不要为了拆分而拆分
3. 运镜和景别要自由选择，符合剧情和人物情绪需要
4. 确保分镜衔接流畅自然，避免遗漏关键剧情点

【镜号规则】
镜号（mirror_id）使用子镜号系统，避免冲突：
- 首次生成分镜：使用 A1, A2, A3... 格式
- 在某镜后新增：使用子镜号，如在 A8 后新增用 A8-1
- 拆分某镜：使用子镜号，如拆分 A9 成3个用 A9-1, A9-2, A9-3
- 合并镜：合并后用新镜号，如合并 A10、A11 后用 A10-1

【特殊操作规则】
当用户要求拆分、插入、删除、合并分镜时：
1. **必须返回完整的分镜列表**（不是只返回新增/修改的分镜）
2. **拆分分镜**：返回完整列表，被拆分的原镜号不要出现
   - 例如：用户说"拆分 A3 成 2 个"，应返回 A1, A2, A3-1, A3-2, A4...（不要包含 A3）
3. **插入分镜**：返回完整列表，新分镜插入到正确位置
   - 例如：在 A8 后插入，应返回 A1, A2, ..., A8, A8-1, A9, A10...
4. **删除分镜**：返回删除后的完整分镜列表
   - 例如：删除 A5，应返回 A1, A2, A3, A4, A6, A7...
5. **合并分镜**：返回完整列表，原镜号不要出现
   - 例如：合并 A8、A9，应返回 A1, ..., A7, A8-1, A10, A11...

【生成图提示词结构说明】
生图提示词分为4层，你只需要填写"动作分镜层"：
- 第1层（全局风格层）和第4层（画质增强层）：由项目配置自动添加
- 第2层（资产锚点层）：在描述中使用 #角色名 或 #场景名 引用，例如 "25岁亚洲男性#张三 坐在沙发上，望向 #客厅 的窗户"
- 第3层（动作分镜层）：你只需填写！格式：景别 + 动作 + 神态 + 位置关系
  示例："Close-up shot, character tilting head slightly, curious expression"

【重要】
- 不要在动作分镜层写风格描述（如"皮克斯风格"）或画质关键词（如"8k"）
- 只描述这个镜头具体发生什么动作、什么神态、什么位置关系
- 用 #角色名/#场景名 来引用资产，系统会自动补充资产描述

【输出格式要求】
必须以 JSON 格式返回，使用以下结构：
```json
{
  "storyboards": [
    {
      "mirror_id": "A1",
      "shot_type": "固定",
      "shot_size": "中景",
      "duration": 3,
      "dialogue": "对白内容",
      "description": "画面描述",
      "notes": "备注",
      "image_prompt_zh": "生图提示词（中文）",
      "image_prompt_en": "生图提示词（英文）",
      "image_prompt_tail_zh": "尾帧提示词（中文）",
      "image_prompt_tail_en": "尾帧提示词（英文）",
      "video_prompt_zh": "视频提示词（中文）",
      "video_prompt_en": "视频提示词（英文）"
    }
  ],
  "characters": [
    {
      "name": "角色名",
      "description": "角色描述",
      "image_prompt_zh": "生图提示词（中文）",
      "image_prompt_en": "生图提示词（英文）",
      "notes": "备注"
    }
  ],
  "scenes": [
    {
      "name": "场景名",
      "description": "场景描述",
      "image_prompt_zh": "生图提示词（中文）",
      "image_prompt_en": "生图提示词（英文）",
      "notes": "备注"
    }
  ],
  "props": [
    {
      "name": "道具名",
      "description": "道具描述",
      "image_prompt_zh": "生图提示词（中文）",
      "image_prompt_en": "生图提示词（英文）",
      "notes": "备注"
    }
  ]
}
```
只返回 JSON 代码块，不要添加任何其他文字说明。"#
        }));
    }

    // 添加历史消息（最多保留最近 10 条）
    if let Some(history) = chat_history {
        let recent_history: Vec<_> = history.into_iter().take(10).collect();
        for msg in recent_history {
            messages.push(json!({
                "role": msg.role,
                "content": msg.content
            }));
        }
    }

    // 添加当前用户消息
    messages.push(json!({
        "role": "user",
        "content": message
    }));

    let request_body = json!({
        "model": model,
        "messages": messages,
        "temperature": 0.7
    });

    let agent = ureq::AgentBuilder::new()
        .timeout_read(std::time::Duration::from_secs(120))
        .timeout_write(std::time::Duration::from_secs(10))
        .build();

    let response = agent.post(&url)
        .set("Authorization", &format!("Bearer {}", api_key))
        .set("Content-Type", "application/json")
        .send_string(
            &serde_json::to_string(&request_body).map_err(|e| e.to_string())?
        )
        .map_err(|e| format!("请求失败: {}", e))?;

    let status = response.status();
    if status >= 400 {
        let error_text = response.into_string()
            .unwrap_or_else(|_| "无法读取错误响应".to_string());
        return Err(format!("API 返回错误 ({}): {}", status, error_text));
    }

    let response_text = response.into_string()
        .map_err(|e| format!("读取响应失败: {}", e))?;

    let response_json: serde_json::Value = serde_json::from_str(&response_text)
        .map_err(|e| format!("解析响应失败: {}", e))?;

    let content = response_json["choices"]
        .get(0)
        .and_then(|c| c["message"]["content"].as_str())
        .unwrap_or("");

    if content.is_empty() {
        return Err("API 返回了空响应".to_string());
    }

    Ok(content.to_string())
}

/// 调用图片生成 API
#[tauri::command]
pub fn call_image_api(api_config: ApiConfig, prompt: String) -> Result<String, String> {
    let base_url = api_config.base_url.trim_end_matches('/');
    let api_key = &api_config.api_key;

    // 根据不同的 API 类型构建不同的请求
    let url = if base_url.contains("openai") || base_url.contains("chatgpt") {
        format!("{}/v1/images/generations", base_url)
    } else {
        // 默认使用 OpenAI 兼容格式
        format!("{}/v1/images/generations", base_url)
    };

    let request_body = json!({
        "model": api_config.model.as_deref().unwrap_or("dall-e-3"),
        "prompt": prompt,
        "n": 1,
        "size": "1024x1024"
    });

    let agent = ureq::AgentBuilder::new()
        .timeout_read(std::time::Duration::from_secs(120))
        .timeout_write(std::time::Duration::from_secs(10))
        .build();

    let response = agent.post(&url)
        .set("Authorization", &format!("Bearer {}", api_key))
        .set("Content-Type", "application/json")
        .send_string(
            &serde_json::to_string(&request_body).map_err(|e| e.to_string())?
        )
        .map_err(|e| format!("图片生成请求失败: {}", e))?;

    let response_text = response.into_string()
        .map_err(|e| format!("读取图片响应失败: {}", e))?;

    let response_json: serde_json::Value = serde_json::from_str(&response_text)
        .map_err(|e| format!("解析图片响应失败: {}", e))?;

    let image_url = response_json["data"]
        .get(0)
        .and_then(|d| d["url"].as_str())
        .or_else(|| response_json["url"].as_str())
        .ok_or_else(|| "无法从响应中提取图片 URL".to_string())?;

    Ok(image_url.to_string())
}

/// 下载图片
#[tauri::command]
pub fn download_image(url: String, save_path: String) -> Result<(), String> {
    let agent = ureq::AgentBuilder::new()
        .timeout_read(std::time::Duration::from_secs(120))
        .build();

    let response = agent.get(&url)
        .call()
        .map_err(|e| format!("下载图片失败: {}", e))?;

    let mut data = Vec::new();
    response.into_reader().read_to_end(&mut data)
        .map_err(|e| format!("读取图片数据失败: {}", e))?;

    fs::write(&save_path, data)
        .map_err(|e| format!("保存图片失败: {}", e))?;

    Ok(())
}

/// 更新分镜图片路径
#[tauri::command]
pub fn update_storyboard_image(
    folder_path: String,
    mirror_id: String,
    image_type: String,
    image_path: String,
) -> Result<(), String> {
    let path = PathBuf::from(&folder_path);
    let db = ProjectDatabase::open(&path)
        .map_err(|e| format!("打开数据库失败: {}", e))?;

    let column = match image_type.as_str() {
        "first" => "image_first_path",
        "last" => "image_last_path",
        _ => return Err("无效的图片类型".to_string()),
    };

    let sql = format!("UPDATE storyboards SET {} = ?1, image_status = 'generated' WHERE mirror_id = ?2", column);

    db.conn().execute(&sql, [&image_path, &mirror_id])
        .map_err(|e| format!("更新分镜图片失败: {}", e))?;

    Ok(())
}

/// 获取项目风格配置
#[tauri::command]
pub fn get_project_style(folder_path: String) -> Result<ProjectStyle, String> {
    let path = PathBuf::from(&folder_path);
    let db = ProjectDatabase::open(&path)
        .map_err(|e| format!("打开数据库失败: {}", e))?;

    let (style_prompt, quality_prompt) = db.get_project_style();

    Ok(ProjectStyle {
        style_prompt,
        quality_prompt,
    })
}

/// 保存项目风格配置
#[tauri::command]
pub fn save_project_style(
    folder_path: String,
    style_prompt: Option<String>,
    quality_prompt: Option<String>,
) -> Result<(), String> {
    let path = PathBuf::from(&folder_path);
    let db = ProjectDatabase::open(&path)
        .map_err(|e| format!("打开数据库失败: {}", e))?;

    db.save_project_style(style_prompt, quality_prompt)
        .map_err(|e| format!("保存项目风格失败: {}", e))?;

    Ok(())
}

/// 保存 Excel 文件
#[tauri::command]
pub fn save_excel_file(folder_path: String) -> Result<String, String> {
    let path = PathBuf::from(&folder_path);
    let project_name = path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("export");

    let file_path = FileDialog::new()
        .set_file_name(&format!("{}.xlsx", project_name))
        .save_file();

    file_path.map(|p| p.to_string_lossy().to_string())
        .ok_or_else(|| "取消保存".to_string())
}

/// 保存 Excel 文件（带对话框）
#[tauri::command]
pub fn save_excel_with_dialog(folder_path: String) -> Result<String, String> {
    save_excel_file(folder_path)
}
