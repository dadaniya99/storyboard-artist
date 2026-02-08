use crate::db::{ProjectDatabase, get_config_dir, get_config_path};
use crate::models::*;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use serde_json::json;

/// 检查并获取全局配置
#[tauri::command]
pub fn get_global_config() -> Result<Option<serde_json::Value>, String> {
    let config_path = get_config_path();

    if !config_path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(config_path)
        .map_err(|e| format!("读取配置失败: {}", e))?;

    let config: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("解析配置失败: {}", e))?;

    Ok(Some(config))
}

/// 保存全局配置
#[tauri::command]
pub fn save_global_config(config: serde_json::Value) -> Result<(), String> {
    let config_dir = get_config_dir();
    let config_path = get_config_path();

    fs::create_dir_all(&config_dir)
        .map_err(|e| format!("创建配置目录失败: {}", e))?;

    let content = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("序列化配置失败: {}", e))?;

    fs::write(config_path, content)
        .map_err(|e| format!("写入配置失败: {}", e))?;

    Ok(())
}

/// 创建新项目
#[tauri::command]
pub fn create_project(folder_path: String, project_name: String) -> Result<String, String> {
    // 检查项目名称是否已存在
    if check_project_name_exists(folder_path.clone(), project_name.clone(), None)? {
        return Err(format!("项目名称 '{}' 已存在，请使用其他名称", project_name));
    }

    let project_path = PathBuf::from(&folder_path).join(&project_name);

    // 检查目录是否存在
    if project_path.exists() {
        return Err(format!("目录 '{}' 已存在", project_name));
    }

    // 创建项目目录
    fs::create_dir_all(&project_path)
        .map_err(|e| format!("创建目录失败: {}", e))?;

    // 创建 .storyboard 目录
    let storyboard_dir = project_path.join(".storyboard");
    fs::create_dir_all(&storyboard_dir)
        .map_err(|e| format!("创建.storyboard目录失败: {}", e))?;

    // 创建 .gitignore
    let gitignore_path = storyboard_dir.join(".gitignore");
    fs::write(gitignore_path, "*\n")
        .map_err(|e| format!("创建.gitignore失败: {}", e))?;

    // 初始化数据库
    let db = ProjectDatabase::open(&project_path)
        .map_err(|e| format!("初始化数据库失败: {}", e))?;

    // 保存项目元数据
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    db.conn().execute(
        "INSERT OR REPLACE INTO project_meta (key, value) VALUES (?1, ?2)",
        [&"name", project_name.as_str()],
    ).map_err(|e| format!("保存项目名称失败: {}", e))?;

    let now_str = now.to_string();
    db.conn().execute(
        "INSERT OR REPLACE INTO project_meta (key, value) VALUES (?1, ?2)",
        [&"created_at", now_str.as_str()],
    ).map_err(|e| format!("保存创建时间失败: {}", e))?;

    db.conn().execute(
        "INSERT OR REPLACE INTO project_meta (key, value) VALUES (?1, ?2)",
        [&"modified_at", now_str.as_str()],
    ).map_err(|e| format!("保存修改时间失败: {}", e))?;

    Ok(project_path.to_string_lossy().to_string())
}

/// 打开项目（验证并加载项目信息）
#[tauri::command]
pub fn open_project(folder_path: String) -> Result<ProjectMeta, String> {
    let project_path = PathBuf::from(&folder_path);

    let db_path = project_path.join(".storyboard").join("project.db");
    if !db_path.exists() {
        return Err("不是有效的分镜师项目".to_string());
    }

    let db = ProjectDatabase::open(&project_path)
        .map_err(|e| format!("打开数据库失败: {}", e))?;

    // 读取项目元数据
    let name: String = db.conn().query_row(
        "SELECT value FROM project_meta WHERE key = ?",
        &[&"name"],
        |row| row.get(0),
    ).unwrap_or_else(|_| "未命名项目".to_string());

    let created_at: i64 = db.conn().query_row(
        "SELECT value FROM project_meta WHERE key = ?",
        &[&"created_at"],
        |row| row.get(0),
    ).unwrap_or(0);

    let modified_at: i64 = db.conn().query_row(
        "SELECT value FROM project_meta WHERE key = ?",
        &[&"modified_at"],
        |row| row.get(0),
    ).unwrap_or(0);

    // 统计分镜数量
    let storyboard_count: i64 = db.conn().query_row(
        "SELECT COUNT(*) FROM storyboards",
        [],
        |row| row.get(0),
    ).unwrap_or(0);

    // 统计对话数量
    let chat_count: i64 = db.conn().query_row(
        "SELECT COUNT(*) FROM chat_history",
        [],
        |row| row.get(0),
    ).unwrap_or(0);

    Ok(ProjectMeta {
        name,
        path: project_path.to_string_lossy().to_string(),
        created_at,
        modified_at,
        storyboard_count,
        chat_count,
    })
}

/// 列出指定目录下的所有项目
#[tauri::command]
pub fn list_projects(folder_path: String) -> Result<Vec<ProjectMeta>, String> {
    let base_path = PathBuf::from(&folder_path);

    if !base_path.exists() {
        return Ok(Vec::new());
    }

    let mut projects = Vec::new();

    let entries = fs::read_dir(&base_path)
        .map_err(|e| format!("读取目录失败: {}", e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("读取目录项失败: {}", e))?;
        let path = entry.path();

        // 只处理目录
        if path.is_dir() {
            let db_path = path.join(".storyboard").join("project.db");
            if db_path.exists() {
                // 这是一个有效的项目
                match open_project(path.to_string_lossy().to_string()) {
                    Ok(meta) => projects.push(meta),
                    Err(_) => continue, // 跳过无效项目
                }
            }
        }
    }

    // 按修改时间降序排序（最近修改的在前）
    projects.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));

    Ok(projects)
}

/// 检查项目名称是否已存在
#[tauri::command]
pub fn check_project_name_exists(folder_path: String, project_name: String, exclude_path: Option<String>) -> Result<bool, String> {
    let projects = list_projects(folder_path)?;

    for project in projects {
        // 排除当前项目（用于重命名时）
        if let Some(ref exclude) = exclude_path {
            if project.path == *exclude {
                continue;
            }
        }

        if project.name == project_name {
            return Ok(true);
        }
    }

    Ok(false)
}

/// 保存 AI 生成的分镜和资产数据
#[tauri::command]
pub fn save_generated_data(
    folder_path: String,
    storyboards: Vec<Storyboard>,
    characters: Vec<Character>,
    scenes: Vec<Scene>,
    props: Vec<Prop>,
    is_regenerate: Option<bool>,
) -> Result<(), String> {
    let project_path = PathBuf::from(&folder_path);
    let db = ProjectDatabase::open(&project_path)
        .map_err(|e| format!("打开数据库失败: {}", e))?;

    let is_regenerate = is_regenerate.unwrap_or(false);

    // 根据操作类型决定清空哪些表
    // 注意：无论是重做还是部分操作（拆分/合并/插入），AI都返回完整分镜列表
    //       因此都需要清空分镜表，然后用AI返回的完整列表替换
    if is_regenerate {
        // 完全重做：清空所有表（分镜+资产）
        db.conn().execute("DELETE FROM storyboards", [])
            .map_err(|e| format!("清空分镜表失败: {}", e))?;
        db.conn().execute("DELETE FROM characters", [])
            .map_err(|e| format!("清空角色表失败: {}", e))?;
        db.conn().execute("DELETE FROM scenes", [])
            .map_err(|e| format!("清空场景表失败: {}", e))?;
        db.conn().execute("DELETE FROM props", [])
            .map_err(|e| format!("清空道具表失败: {}", e))?;
    } else {
        // 部分操作（拆分/合并/插入）：只清空分镜表，保留资产表
        // 原因：AI返回完整分镜列表，需要完全替换现有分镜（包括删除不在新列表中的）
        db.conn().execute("DELETE FROM storyboards", [])
            .map_err(|e| format!("清空分镜表失败: {}", e))?;
    }

    // 序号始终从1开始，因为AI返回的是完整分镜列表
    let seq_start = 1;

    // 插入分镜数据
    // 注意：忽略 AI 返回的 sequence_number，根据 AI 返回的顺序重新分配序号
    //       mirror_id 是主键，放在前面
    for (index, storyboard) in storyboards.iter().enumerate() {
        let new_seq = (seq_start + index as i64).to_string();

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
                &new_seq,
                &storyboard.shot_type.clone().unwrap_or_default(),
                &storyboard.shot_size.clone().unwrap_or_default(),
                &storyboard.duration.map(|d| d.to_string()).unwrap_or_default(),
                &storyboard.dialogue.clone().unwrap_or_default(),
                &storyboard.description.clone().unwrap_or_default(),
                &storyboard.notes.clone().unwrap_or_default(),
                &storyboard.image_prompt_zh.clone().unwrap_or_default(),
                &storyboard.image_prompt_en.clone().unwrap_or_default(),
                &storyboard.image_prompt_tail_zh.clone().unwrap_or_default(),
                &storyboard.image_prompt_tail_en.clone().unwrap_or_default(),
                &storyboard.video_prompt_zh.clone().unwrap_or_default(),
                &storyboard.video_prompt_en.clone().unwrap_or_default(),
            ],
        ).map_err(|e| format!("插入分镜失败: {}", e))?;
    }

    // 插入角色数据（使用 INSERT OR IGNORE 避免重复）
    for character in characters {
        db.conn().execute(
            "INSERT OR IGNORE INTO characters (name, description, image_prompt_zh, image_prompt_en, notes)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            [
                &character.name,
                &character.description.unwrap_or_default(),
                &character.image_prompt_zh.unwrap_or_default(),
                &character.image_prompt_en.unwrap_or_default(),
                &character.notes.unwrap_or_default(),
            ],
        ).map_err(|e| format!("插入角色失败: {}", e))?;
    }

    // 插入场景数据（使用 INSERT OR IGNORE 避免重复）
    for scene in scenes {
        db.conn().execute(
            "INSERT OR IGNORE INTO scenes (name, description, image_prompt_zh, image_prompt_en, notes)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            [
                &scene.name,
                &scene.description.unwrap_or_default(),
                &scene.image_prompt_zh.unwrap_or_default(),
                &scene.image_prompt_en.unwrap_or_default(),
                &scene.notes.unwrap_or_default(),
            ],
        ).map_err(|e| format!("插入场景失败: {}", e))?;
    }

    // 插入道具数据（使用 INSERT OR IGNORE 避免重复）
    for prop in props {
        db.conn().execute(
            "INSERT OR IGNORE INTO props (name, description, image_prompt_zh, image_prompt_en, notes)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            [
                &prop.name,
                &prop.description.unwrap_or_default(),
                &prop.image_prompt_zh.unwrap_or_default(),
                &prop.image_prompt_en.unwrap_or_default(),
                &prop.notes.unwrap_or_default(),
            ],
        ).map_err(|e| format!("插入道具失败: {}", e))?;
    }

    // 更新修改时间
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    db.conn().execute(
        "UPDATE project_meta SET value = ? WHERE key = 'modified_at'",
        &[&now.to_string()],
    ).map_err(|e| format!("更新修改时间失败: {}", e))?;

    Ok(())
}

/// 获取分镜表数据
#[tauri::command]
pub fn get_storyboards(folder_path: String) -> Result<Vec<Storyboard>, String> {
    let project_path = PathBuf::from(&folder_path);
    let db = ProjectDatabase::open(&project_path)
        .map_err(|e| format!("打开数据库失败: {}", e))?;

    // 使用 CAST 将 sequence_number 转为整数排序，避免字符串排序导致 "10" 排在 "2" 前面
    let mut stmt = db.conn().prepare(
        "SELECT sequence_number, mirror_id, shot_type, shot_size, duration,
                dialogue, description, notes,
                image_prompt_zh, image_prompt_en,
                image_prompt_tail_zh, image_prompt_tail_en,
                video_prompt_zh, video_prompt_en
         FROM storyboards ORDER BY CAST(sequence_number AS INTEGER)"
    ).map_err(|e| format!("准备查询失败: {}", e))?;

    let rows = stmt.query_map([], |row| {
        Ok(Storyboard {
            sequence_number: row.get(0)?,
            mirror_id: row.get(1)?,
            shot_type: row.get(2)?,
            shot_size: row.get(3)?,
            duration: row.get(4)?,
            dialogue: row.get(5)?,
            description: row.get(6)?,
            notes: row.get(7)?,
            image_prompt_zh: row.get(8)?,
            image_prompt_en: row.get(9)?,
            image_prompt_tail_zh: row.get(10)?,
            image_prompt_tail_en: row.get(11)?,
            video_prompt_zh: row.get(12)?,
            video_prompt_en: row.get(13)?,
        })
    }).map_err(|e| format!("查询分镜失败: {}", e))?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row.map_err(|e| format!("解析分镜失败: {}", e))?);
    }

    Ok(result)
}

/// 获取角色资产数据
#[tauri::command]
pub fn get_characters(folder_path: String) -> Result<Vec<Character>, String> {
    let project_path = PathBuf::from(&folder_path);
    let db = ProjectDatabase::open(&project_path)
        .map_err(|e| format!("打开数据库失败: {}", e))?;

    let mut stmt = db.conn().prepare(
        "SELECT name, description, image_prompt_zh, image_prompt_en, notes
         FROM characters ORDER BY name"
    ).map_err(|e| format!("准备查询失败: {}", e))?;

    let rows = stmt.query_map([], |row| {
        Ok(Character {
            name: row.get(0)?,
            description: row.get(1)?,
            image_prompt_zh: row.get(2)?,
            image_prompt_en: row.get(3)?,
            notes: row.get(4)?,
        })
    }).map_err(|e| format!("查询角色失败: {}", e))?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row.map_err(|e| format!("解析角色失败: {}", e))?);
    }

    Ok(result)
}

/// 获取场景资产数据
#[tauri::command]
pub fn get_scenes(folder_path: String) -> Result<Vec<Scene>, String> {
    let project_path = PathBuf::from(&folder_path);
    let db = ProjectDatabase::open(&project_path)
        .map_err(|e| format!("打开数据库失败: {}", e))?;

    let mut stmt = db.conn().prepare(
        "SELECT name, description, image_prompt_zh, image_prompt_en, notes
         FROM scenes ORDER BY name"
    ).map_err(|e| format!("准备查询失败: {}", e))?;

    let rows = stmt.query_map([], |row| {
        Ok(Scene {
            name: row.get(0)?,
            description: row.get(1)?,
            image_prompt_zh: row.get(2)?,
            image_prompt_en: row.get(3)?,
            notes: row.get(4)?,
        })
    }).map_err(|e| format!("查询场景失败: {}", e))?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row.map_err(|e| format!("解析场景失败: {}", e))?);
    }

    Ok(result)
}

/// 获取道具资产数据
#[tauri::command]
pub fn get_props(folder_path: String) -> Result<Vec<Prop>, String> {
    let project_path = PathBuf::from(&folder_path);
    let db = ProjectDatabase::open(&project_path)
        .map_err(|e| format!("打开数据库失败: {}", e))?;

    let mut stmt = db.conn().prepare(
        "SELECT name, description, image_prompt_zh, image_prompt_en, notes
         FROM props ORDER BY name"
    ).map_err(|e| format!("准备查询失败: {}", e))?;

    let rows = stmt.query_map([], |row| {
        Ok(Prop {
            name: row.get(0)?,
            description: row.get(1)?,
            image_prompt_zh: row.get(2)?,
            image_prompt_en: row.get(3)?,
            notes: row.get(4)?,
        })
    }).map_err(|e| format!("查询道具失败: {}", e))?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row.map_err(|e| format!("解析道具失败: {}", e))?);
    }

    Ok(result)
}

/// 保存聊天消息
#[tauri::command]
pub fn save_chat_message(folder_path: String, role: String, content: String) -> Result<(), String> {
    let project_path = PathBuf::from(&folder_path);
    let db = ProjectDatabase::open(&project_path)
        .map_err(|e| format!("打开数据库失败: {}", e))?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    db.conn().execute(
        "INSERT INTO chat_history (role, content, timestamp) VALUES (?1, ?2, ?3)",
        &[&role, &content, &now.to_string()],
    ).map_err(|e| format!("保存聊天消息失败: {}", e))?;

    Ok(())
}

/// 获取聊天历史
#[tauri::command]
pub fn get_chat_history(folder_path: String, limit: Option<i64>) -> Result<Vec<ChatMessage>, String> {
    let project_path = PathBuf::from(&folder_path);
    let db = ProjectDatabase::open(&project_path)
        .map_err(|e| format!("打开数据库失败: {}", e))?;

    let limit = limit.unwrap_or(50);

    let mut stmt = db.conn().prepare(
        "SELECT id, role, content, timestamp FROM chat_history ORDER BY id DESC LIMIT ?"
    ).map_err(|e| format!("准备查询失败: {}", e))?;

    let rows = stmt.query_map([&limit], |row| {
        Ok(ChatMessage {
            id: Some(row.get(0)?),
            role: row.get(1)?,
            content: row.get(2)?,
            timestamp: Some(row.get(3)?),
        })
    }).map_err(|e| format!("查询聊天历史失败: {}", e))?;

    let mut result: Vec<ChatMessage> = Vec::new();
    for row in rows {
        result.push(row.map_err(|e| format!("解析聊天消息失败: {}", e))?);
    }

    // 反转顺序，使最新的在最后
    result.reverse();
    Ok(result)
}

/// 更新项目名称
#[tauri::command]
pub fn update_project_name(folder_path: String, name: String) -> Result<(), String> {
    let project_path = PathBuf::from(&folder_path);

    // 获取父目录路径用于检查重名
    let parent_folder = project_path.parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| String::from("D:\\分镜项目"));

    // 检查项目名称是否已存在（排除当前项目）
    if check_project_name_exists(parent_folder, name.clone(), Some(folder_path.clone()))? {
        return Err(format!("项目名称 '{}' 已存在，请使用其他名称", name));
    }

    let db = ProjectDatabase::open(&project_path)
        .map_err(|e| format!("打开数据库失败: {}", e))?;

    // 更新项目名称
    db.conn().execute(
        "INSERT OR REPLACE INTO project_meta (key, value) VALUES (?1, ?2)",
        [&"name", name.as_str()],
    ).map_err(|e| format!("更新项目名称失败: {}", e))?;

    // 更新修改时间
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    db.conn().execute(
        "UPDATE project_meta SET value = ? WHERE key = 'modified_at'",
        &[&now.to_string()],
    ).map_err(|e| format!("更新修改时间失败: {}", e))?;

    Ok(())
}

/// 调用 AI API
#[tauri::command]
pub fn call_ai_api(api_config: ApiConfig, message: String, chat_history: Option<Vec<ChatMessage>>) -> Result<String, String> {
    // 解析 API 配置
    let base_url = api_config.base_url.trim_end_matches('/');
    let api_key = &api_config.api_key;
    let model = api_config.model.as_deref().unwrap_or("gpt-3.5-turbo");

    // 构建 OpenAI 兼容的请求
    let url = format!("{}/chat/completions", base_url);

    // 构建消息数组
    let mut messages = Vec::new();

    // 系统提示词
    messages.push(json!({
        "role": "system",
        "content": r#"你是一位拥有10年影视动画经验的资深职业分镜师，擅长镜头语言、叙事节奏、画面构图、运镜设计、剪辑逻辑。你的任务是把用户提供的剧本/文案/情节，严格转换成标准分镜脚本，遵循电影语言规范，不抒情、不文艺化、不脑补无关剧情，只做专业、可落地、可拍摄的分镜设计。

【分镜设计原则】
1. 镜头拆分要充分细致，基于叙事节奏，每个镜头要有明确的目的和叙事功能
2. 遵循"一镜一意"原则，避免一个镜头表达过多信息
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

【输出格式】
当用户要求生成分镜时，请按以下 JSON 格式输出。必须生成所有字段，不能省略。
注意：sequence_number 字段请填写 0，系统会自动重新分配正确的序号。
{
  "storyboards": [
    {
      "sequence_number": 0,
      "mirror_id": "A1",
      "shot_type": "运镜方式（如：推/拉/摇/移/固定等）",
      "shot_size": "景别（如：远景/全景/中景/近景/特写等）",
      "duration": 3,
      "dialogue": "台词内容",
      "description": "画面描述",
      "notes": "备注信息",
      "image_prompt_zh": "首帧画面提示词（中文）",
      "image_prompt_en": "First frame image prompt in English, detailed and professional",
      "image_prompt_tail_zh": "尾帧画面提示词（中文）",
      "image_prompt_tail_en": "Last frame image prompt in English",
      "video_prompt_zh": "视频生成提示词（中文）",
      "video_prompt_en": "Video generation prompt in English"
    }
  ],
  "characters": [{"name": "角色名", "description": "外貌描述", "image_prompt_zh": "角色图像提示词", "image_prompt_en": "Character image prompt", "notes": "备注"}],
  "scenes": [{"name": "场景名", "description": "场景描述", "image_prompt_zh": "场景图像提示词", "image_prompt_en": "Scene image prompt", "notes": "备注"}],
  "props": [{"name": "道具名", "description": "道具描述", "image_prompt_zh": "道具图像提示词", "image_prompt_en": "Prop image prompt", "notes": "备注"}]
}

【重要提示】
- 必须生成完整的 JSON，包含所有字段
- sequence_number 字段请固定填写 0，系统会自动分配正确的序号（无需关注）
- mirror_id 必须使用子镜号规则避免冲突（A1, A8-1, A9-2...）
- image_prompt_tail_zh/en 和 video_prompt_zh/en 不能省略
- 英文提示词要专业、详细，适合 AI 图像/视频生成
- 分镜要充分拆分，确保每个关键动作和情绪都有对应的镜头
- shot_type 和 shot_size 字段必须有值"#
    }));

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

    // 使用 ureq 发送同步请求，设置超时
    let agent = ureq::AgentBuilder::new()
        .timeout_read(std::time::Duration::from_secs(120))
        .timeout_write(std::time::Duration::from_secs(10))
        .build();

    let response = agent.post(&url)
        .set("Authorization", &format!("Bearer {}", api_key))
        .set("Content-Type", "application/json")
        .send_string(
            &serde_json::to_string(&request_body).map_err(|e| format!("序列化请求失败: {}", e))?
        )
        .map_err(|e| format!("请求失败: {}", e))?;

    let status = response.status();
    if status >= 400 {
        let error_text = response.into_string()
            .unwrap_or_else(|_| "无法读取错误响应".to_string());
        return Err(format!("API 返回错误 ({}): {}", status, error_text));
    }

    // 读取响应
    let response_text = response.into_string()
        .map_err(|e| format!("读取响应失败: {}", e))?;

    // 解析响应
    let response_json: serde_json::Value = serde_json::from_str(&response_text)
        .map_err(|e| format!("解析响应失败: {}", e))?;

    // 提取消息内容
    let content = response_json["choices"]
        .get(0)
        .and_then(|c| c["message"]["content"].as_str())
        .unwrap_or("");

    if content.is_empty() {
        return Err("API 返回了空响应".to_string());
    }

    Ok(content.to_string())
}
