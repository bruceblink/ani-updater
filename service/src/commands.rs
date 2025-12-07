use common::api::ApiResponse;
use common::po::ItemResult;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::process_news_info_to_item::{extract_news_keywords, query_news_info_to_extract};
use crate::spider::agedm::fetch_agedm_ani_data;
use crate::spider::bilibili::fetch_bilibili_ani_data;
use crate::spider::douban::fetch_douban_movie_data;
use crate::spider::health_checker::health_check;
use crate::spider::iqiyi::fetch_iqiyi_ani_data;
use crate::spider::mikanani::fetch_mikanani_ani_data;
use crate::spider::news::fetch_latest_news_data;
use crate::spider::tencent::fetch_qq_ani_data;
use crate::spider::youku::fetch_youku_ani_data;
use sqlx::PgPool;

/// 通用命令输入参数，可以传任意 JSON 数据，也可传数据库连接
#[derive(Clone)]
pub struct CommandInput {
    pub args: String,
    pub db: Option<Arc<PgPool>>, // 可选数据库连接
}

/// CmdFn 表示：接收 CommandInput，返回一个 boxed future，输出为 Result<ApiResponse<ItemResult>, String>
pub type CmdFn = Arc<
    dyn Fn(
            CommandInput,
        ) -> Pin<Box<dyn Future<Output = Result<ApiResponse<ItemResult>, String>> + Send>>
        + Send
        + Sync,
>;

/// 构建命令表，将异步函数包装成 CmdFn
pub fn build_cmd_map() -> HashMap<String, CmdFn> {
    let mut map: HashMap<String, CmdFn> = HashMap::new();

    // 所有爬虫函数
    map.insert(
        "fetch_bilibili_ani_data".to_string(),
        Arc::new(|input: CommandInput| Box::pin(fetch_bilibili_ani_data(input.args))),
    );

    map.insert(
        "fetch_iqiyi_ani_data".to_string(),
        Arc::new(|input: CommandInput| Box::pin(fetch_iqiyi_ani_data(input.args))),
    );

    map.insert(
        "fetch_mikanani_ani_data".to_string(),
        Arc::new(|input: CommandInput| Box::pin(fetch_mikanani_ani_data(input.args))),
    );

    map.insert(
        "fetch_qq_ani_data".to_string(),
        Arc::new(|input: CommandInput| Box::pin(fetch_qq_ani_data(input.args))),
    );

    map.insert(
        "fetch_youku_ani_data".to_string(),
        Arc::new(|input: CommandInput| Box::pin(fetch_youku_ani_data(input.args))),
    );

    map.insert(
        "fetch_agedm_ani_data".to_string(),
        Arc::new(|input: CommandInput| Box::pin(fetch_agedm_ani_data(input.args))),
    );

    map.insert(
        "fetch_douban_movie_data".to_string(),
        Arc::new(|input: CommandInput| Box::pin(fetch_douban_movie_data(input.args))),
    );

    map.insert(
        "fetch_latest_news_data".to_string(),
        Arc::new(|input: CommandInput| Box::pin(fetch_latest_news_data(input.args))),
    );

    map.insert(
        "health_check".to_string(),
        Arc::new(|input: CommandInput| Box::pin(health_check(input.args))),
    );

    map.insert(
        "extract_transform_news_info_to_item".to_string(),
        Arc::new(|input: CommandInput| match input.db.clone() {
            Some(db_pool) => Box::pin(query_news_info_to_extract(db_pool)),
            None => Box::pin(async { Err("DbPool is required".to_string()) }),
        }),
    );

    map.insert(
        "extract_keywords_to_news_keywords".to_string(),
        Arc::new(|input: CommandInput| Box::pin(extract_news_keywords(input.args))),
    );

    map
}
