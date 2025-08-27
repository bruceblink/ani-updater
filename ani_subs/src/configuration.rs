use crate::timer_tasker::task::TaskMeta;
use secrecy::{ExposeSecret, Secret};
use serde_aux::field_attributes::deserialize_number_from_string;
use sqlx::postgres::{PgConnectOptions, PgSslMode};
use std::collections::HashMap;
use std::convert::TryInto;
use std::fs;
use std::path::PathBuf;
use tracing::info;

#[derive(serde::Deserialize)]
pub struct Setting {
    pub database: DatabaseSettings,
    pub application: ApplicationSettings,
    // email_client
    pub email_client: EmailClientSettings,
    pub datasource: TaskConfig,
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

// 读取配置文件
pub fn get_configuration() -> Result<Setting, config::ConfigError> {
    // 获取根目录路径
    let base_path = std::env::current_dir().expect("Failed to determine the current directory");
    // 读取配置文件目录
    let configuration_directory = base_path.join("configuration");
    // 检查运行时环境，如果没有指定则默认是"local"
    let environment: Environment = std::env::var("APP_ENVIRONMENT")
        .unwrap_or_else(|_| "local".into())
        .try_into()
        .expect("Failed to parse APP_ENVIRONMENT.");
    let environment_filename = format!("{}.yaml", environment.as_str());
    let settings = config::Config::builder()
        .add_source(config::File::from(
            configuration_directory.join("base.yaml"),
        ))
        .add_source(config::File::from(
            configuration_directory.join(&environment_filename),
        ))
        // 从环境变量中添加设置,例如，通过 APP_APPLICATION__PORT可以设置为 Settings.application.port
        .add_source(
            config::Environment::with_prefix("APP")
                .prefix_separator("_")
                .separator("__"),
        )
        .build()?;
    settings.try_deserialize::<Setting>()
}

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
}

impl TryFrom<String> for Environment {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        match s.to_lowercase().as_str() {
            "local" => Ok(Self::Local),
            "production" => Ok(Self::Production),
            other => Err(format!(
                "{other} is not a supported environment. Use either `local` or `production`. "
            )),
        }
    }
}

pub type TaskConfig = HashMap<String, Vec<TaskMeta>>;

/// 初始化应用配置
pub fn init_config() -> std::io::Result<PathBuf> {
    let app_path = std::env::current_dir().expect("Failed to determine the current directory"); // 应用程序的根目录
    // 配置文件目录
    let config_path = app_path.join("conf");
    // 配置文件的目标路径
    let target_config_path = config_path.join("tasks.yaml");

    // 检查配置文件是否已存在，如果不存在则复制
    if !target_config_path.exists() {
        let config_file_in_resources = config_path.join("conf").join("tasks.yaml");

        // 复制配置文件到目标目录
        if !config_file_in_resources.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Config file not found in resources",
            ));
        }

        // 复制文件
        fs::create_dir_all(config_path.clone())?; // 如果目标目录不存在则创建
        fs::copy(config_file_in_resources, target_config_path)?;
        info!("配置文件已复制到目标目录");
    } else {
        info!("配置文件已存在");
    }

    Ok(config_path)
}
