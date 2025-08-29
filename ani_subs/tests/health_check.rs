use ani_subs::configuration::{DatabaseSettings, get_configuration};
use ani_subs::dao::insert_users;
use ani_subs::domain::dto::NewUser;
use ani_subs::startup::run;
use ani_subs::telemetry::{get_subscriber, init_subscriber};
use secrecy::Secret;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::LazyLock;
use uuid::Uuid;

// Ensure that the `tracing` stack is only initialised once using `once_cell`
static TRACING: LazyLock<()> = LazyLock::new(|| {
    let default_filter_level = "info".to_string();
    let subscriber_name = "test".to_string();
    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::stdout);
        init_subscriber(subscriber);
    } else {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::sink);
        init_subscriber(subscriber);
    };
});

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
}

async fn spawn_app() -> TestApp {
    // The first time `initialize` is invoked the code in `TRACING` is executed.
    // All other invocations will instead skip execution.
    LazyLock::force(&TRACING);

    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port");
    // We retrieve the port assigned to us by the OS
    let port = listener.local_addr().unwrap().port();
    let address = format!("http://127.0.0.1:{}", port);

    let mut configuration = get_configuration(Some(PathBuf::from("../configuration")))
        .expect("Failed to read configuration.");
    configuration.database.database_name = Uuid::new_v4().to_string();
    let connection_pool = configure_database(&configuration.database).await;

    let server = run(listener, connection_pool.clone()).expect("Failed to bind address");
    let _ = tokio::spawn(server);
    TestApp {
        address,
        db_pool: connection_pool,
    }
}

pub async fn configure_database(config: &DatabaseSettings) -> PgPool {
    // Create database
    let maintenance_settings = DatabaseSettings {
        database_name: "postgres".to_string(),
        username: "postgres".to_string(),
        password: Secret::new("password".to_string()),
        ..config.clone()
    };
    let mut connection = PgConnection::connect_with(&maintenance_settings.connect_options())
        .await
        .expect("Failed to connect to Postgres");
    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, config.database_name).as_str())
        .await
        .expect("Failed to create database.");

    // Migrate database
    let connection_pool = PgPool::connect_with(config.connect_options())
        .await
        .expect("Failed to connect to Postgres.");
    sqlx::migrate!("../migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to migrate the database");
    connection_pool
}

#[tokio::test]
async fn health_check_works() {
    // Arrange
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    // Act
    let response = client
        // Use the returned application address
        .get(&format!("{}/health_check", &app.address))
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

#[tokio::test]
async fn test_get_ani_info() {
    // Arrange
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    // Act
    let response = client
        // Use the returned application address
        .get(&format!("{}/anis/{}", &app.address, 1))
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert_eq!(response.status().as_u16(), 404);
    assert_eq!(Some(0), response.content_length());
}

#[tokio::test]
async fn test_get_ani_info_list() {
    // Arrange
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    // Act
    let response = client
        // Use the returned application address
        .get(&format!("{}/anis", &app.address))
        .send()
        .await
        .expect("Failed to execute request.");
    // Assert
    assert_eq!(response.status().as_u16(), 200);
}

#[tokio::test]
async fn test_login_get_token() {
    // Arrange
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    let test_users = build_test_users();

    let _ = insert_users(&test_users, &app.db_pool).await;

    let login_body = serde_json::json!({
        "username": "bob",
        "password": "securepass"
    });

    // Act
    let response = client
        .post(&format!("{}/login", &app.address))
        .json(&login_body) // 这里设置 JSON body
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert_eq!(response.status().as_u16(), 401);

    // 可选：解析返回 JSON
    //let body: serde_json::Value = response.json().await.unwrap();
    println!("Response body: {:?}", response);
}

pub fn build_test_users() -> Vec<NewUser> {
    vec![
        NewUser {
            email: "alice@example.com".to_string(),
            username: "alice".to_string(),
            password: "password123".to_string(),
            display_name: "Alice".to_string(),
            avatar_url: "https://example.com/avatar/alice.png".to_string(),
        },
        NewUser {
            email: "bob@example.com".to_string(),
            username: "bob".to_string(),
            password: "securepass".to_string(),
            display_name: "Bob".to_string(),
            avatar_url: "https://example.com/avatar/bob.png".to_string(),
        },
        NewUser {
            email: "charlie@example.com".to_string(),
            username: "charlie".to_string(),
            password: "charliepw".to_string(),
            display_name: "Charlie".to_string(),
            avatar_url: "https://example.com/avatar/charlie.png".to_string(),
        },
    ]
}
