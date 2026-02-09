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
        // 先检查是否需要迁移旧数据库格式
        self.migrate_old_schema()?;

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

    /// 迁移旧数据库格式到新格式
    fn migrate_old_schema(&self) -> SqliteResult<()> {
        // 检查 storyboards 表是否存在及其结构
        let (table_exists, has_old_id_column): (bool, bool) = match self.conn().prepare(
            "SELECT sql FROM sqlite_master WHERE type='table' AND name='storyboards'"
        ) {
            Ok(mut stmt) => {
                match stmt.query_row([], |row| -> Result<(bool, bool), rusqlite::Error> {
                    let schema: String = row.get(0)?;
                    // 检查是否有 id INTEGER PRIMARY KEY（旧格式）
                    let has_id = schema.contains("id INTEGER PRIMARY KEY");
                    // 检查是否已经有 mirror_id TEXT PRIMARY KEY（新格式）
                    let has_mirror_id = schema.contains("mirror_id TEXT PRIMARY KEY");
                    Ok((true, has_id && !has_mirror_id))
                }) {
                    Ok(result) => result,
                    Err(_) => (false, false),
                }
            }
            Err(_) => (false, false),
        };

        if table_exists && has_old_id_column {
            eprintln!("检测到旧数据库格式，开始迁移...");

            // 1. 创建新表
            self.conn.execute(
                "CREATE TABLE storyboards_new (
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

            // 2. 检查旧表是否有 mirror_id 列
            let has_mirror_id_column: bool = match self.conn().query_row(
                "SELECT COUNT(*) FROM pragma_table_info('storyboards') WHERE name='mirror_id'",
                [],
                |row| row.get::<_, i64>(0),
            ) {
                Ok(count) => count > 0,
                Err(_) => false,
            };

            // 3. 复制数据
            let copy_result = if has_mirror_id_column {
                // 旧表有 mirror_id 列，直接使用
                self.conn.execute(
                    "INSERT INTO storyboards_new (
                        mirror_id, sequence_number, shot_type, shot_size, duration,
                        dialogue, description, notes,
                        image_prompt_zh, image_prompt_en,
                        image_prompt_tail_zh, image_prompt_tail_en,
                        video_prompt_zh, video_prompt_en
                    )
                    SELECT
                        COALESCE(mirror_id, CAST(id AS TEXT), '') AS mirror_id,
                        COALESCE(sequence_number, 0) AS sequence_number,
                        shot_type, shot_size, duration,
                        dialogue, description, notes,
                        image_prompt_zh, image_prompt_en,
                        image_prompt_tail_zh, image_prompt_tail_en,
                        video_prompt_zh, video_prompt_en
                    FROM storyboards",
                    [],
                )
            } else {
                // 旧表没有 mirror_id 列，用 id 生成
                self.conn.execute(
                    "INSERT INTO storyboards_new (
                        mirror_id, sequence_number, shot_type, shot_size, duration,
                        dialogue, description, notes,
                        image_prompt_zh, image_prompt_en,
                        image_prompt_tail_zh, image_prompt_tail_en,
                        video_prompt_zh, video_prompt_en
                    )
                    SELECT
                        CAST(id AS TEXT) AS mirror_id,
                        COALESCE(sequence_number, 0) AS sequence_number,
                        shot_type, shot_size, duration,
                        dialogue, description, notes,
                        image_prompt_zh, image_prompt_en,
                        image_prompt_tail_zh, image_prompt_tail_en,
                        video_prompt_zh, video_prompt_en
                    FROM storyboards",
                    [],
                )
            };

            if copy_result.is_ok() {
                // 4. 删除旧表
                let _ = self.conn.execute("DROP TABLE storyboards", []);
                // 5. 重命名新表
                let _ = self.conn.execute("ALTER TABLE storyboards_new RENAME TO storyboards", []);
                eprintln!("数据库迁移完成");
            } else {
                eprintln!("数据迁移失败: {:?}", copy_result);
                // 清理临时表
                let _ = self.conn.execute("DROP TABLE IF EXISTS storyboards_new", []);
            }
        }

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
