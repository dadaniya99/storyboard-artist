use rusqlite::{Connection, Result as SqliteResult};
use std::path::PathBuf;
use dirs::home_dir;

/// 全局配置路径
pub fn get_config_dir() -> PathBuf {
    home_dir()
        .expect("无法找到用户目录")
        .join(".storyboard")
}

/// 获取全局配置文件路径
pub fn get_config_path() -> PathBuf {
    get_config_dir().join("config.json")
}

/// 项目数据库管理器
pub struct ProjectDatabase {
    conn: Connection,
}

impl ProjectDatabase {
    /// 打开或创建项目数据库
    pub fn open(project_path: &PathBuf) -> SqliteResult<Self> {
        let db_path = project_path.join(".storyboard").join("project.db");

        // 确保 .storyboard 目录存在
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        let conn = Connection::open(db_path)?;

        let db = ProjectDatabase { conn };
        db.init_tables()?;
        Ok(db)
    }

    /// 初始化数据库表
    fn init_tables(&self) -> SqliteResult<()> {
        // 分镜表 (storyboards)
        // 注意：mirror_id 是主键（镜号唯一、不可修改）
        //       sequence_number 只是排序用的序号，每次操作后重新编号
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS storyboards (
                mirror_id TEXT PRIMARY KEY,
                sequence_number INTEGER NOT NULL,
                shot_type TEXT,
                shot_size TEXT,
                duration REAL,
                dialogue TEXT,
                description TEXT,
                notes TEXT,
                image_prompt_zh TEXT,
                image_prompt_en TEXT,
                image_prompt_tail_zh TEXT,
                image_prompt_tail_en TEXT,
                video_prompt_zh TEXT,
                video_prompt_en TEXT
            )",
            [],
        )?;

        // 角色资产表 (characters)
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS characters (
                name TEXT PRIMARY KEY,
                description TEXT,
                image_prompt_zh TEXT,
                image_prompt_en TEXT,
                notes TEXT
            )",
            [],
        )?;

        // 场景资产表 (scenes)
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS scenes (
                name TEXT PRIMARY KEY,
                description TEXT,
                image_prompt_zh TEXT,
                image_prompt_en TEXT,
                notes TEXT
            )",
            [],
        )?;

        // 道具资产表 (props)
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS props (
                name TEXT PRIMARY KEY,
                description TEXT,
                image_prompt_zh TEXT,
                image_prompt_en TEXT,
                notes TEXT
            )",
            [],
        )?;

        // 项目元数据表 (project_meta)
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS project_meta (
                key TEXT PRIMARY KEY,
                value TEXT
            )",
            [],
        )?;

        // AI对话历史表 (chat_history)
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS chat_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                timestamp INTEGER NOT NULL
            )",
            [],
        )?;

        Ok(())
    }

    /// 获取数据库连接引用
    pub fn conn(&self) -> &Connection {
        &self.conn
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_dir() {
        let config_dir = get_config_dir();
        assert!(config_dir.ends_with(".storyboard"));
    }
}
