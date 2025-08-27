use secrecy::{ExposeSecret, Secret};
use serde_aux::field_attributes::deserialize_number_from_string;
use sqlx::postgres::{PgConnectOptions, PgSslMode};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(serde::Deserialize)]
pub struct Setting {
    pub database: DatabaseSettings,
    pub application: ApplicationSettings,
    // email_client
    pub email_client: EmailClientSettings,
    pub task_config: TaskConfig,
}

#[derive(serde::Deserialize)]
pub struct EmailClientSettings {
    pub base_url: String,
    pub sender_email: String,
    // 新的密钥配置项
    pub authorization_token: Secret<String>,
}

#[derive(serde::Deserialize)]
pub struct ApplicationSettings {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub host: String,
}

#[derive(serde::Deserialize, Clone)]
pub struct DatabaseSettings {
    pub username: String,
    pub password: Secret<String>,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub host: String,
    pub database_name: String,
    pub require_ssl: bool,
}

impl DatabaseSettings {
    pub fn connect_options(&self) -> PgConnectOptions {
        let ssl_mode = if self.require_ssl {
            PgSslMode::Require
        } else {
            PgSslMode::Prefer
        };
        PgConnectOptions::new()
            .host(&self.host)
            .username(&self.username)
            .password(self.password.expose_secret())
            .port(self.port)
            .ssl_mode(ssl_mode)
            .database(&self.database_name)
    }
}

pub type TaskConfig = HashMap<String, Vec<crate::timer_tasker::task::TaskMeta>>;

/// ------------------------ 环境 ------------------------
pub enum Environment {
    Local,
    Production,
}

impl Environment {
    pub fn as_str(&self) -> &'static str {
        match self {
            Environment::Local => "local",
            Environment::Production => "production",
        }
    }

    /// 根据字符串解析环境
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "local" => Environment::Local,
            "production" => Environment::Production,
            _ => Environment::Production, // 默认生产环境
        }
    }
}

/// ------------------------ 配置加载 ------------------------
pub fn get_configuration(
    config_dir: Option<PathBuf>,      // 可选覆盖目录
    environment: Option<Environment>, // 可选覆盖环境
) -> Result<Setting, config::ConfigError> {
    // 配置目录默认使用 crate 根目录下的 configuration
    let config_directory = config_dir
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("configuration"));

    // 环境默认 production
    let env = environment.unwrap_or(Environment::Production);
    let env_filename = format!("{}.yaml", env.as_str());

    // 构建配置
    let settings = config::Config::builder()
        .add_source(config::File::from(config_directory.join("base.yaml")))
        .add_source(config::File::from(config_directory.join(env_filename)))
        .add_source(
            config::Environment::with_prefix("APP")
                .prefix_separator("_")
                .separator("__"),
        )
        .build()?;

    settings.try_deserialize::<Setting>()
}
