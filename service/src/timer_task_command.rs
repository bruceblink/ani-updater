use common::api::ApiResponse;
use common::po::ItemResult;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::health_checker::health_check;
use crate::process_news_info::{
    extract_news_event, extract_news_item, extract_news_keywords, fetch_all_news,
    merge_cross_day_news_events,
};

/// 通用命令输入参数，可以传任意 JSON 数据
#[derive(Clone)]
pub struct CommandInput {
    pub urls: Option<String>,
    pub args: String,
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

    map.insert(
        "health_check".to_string(),
        Arc::new(|input: CommandInput| Box::pin(health_check(input.args))),
    );

    map.insert(
        "extract_transform_news_info_to_item".to_string(),
        Arc::new(|input: CommandInput| Box::pin(extract_news_item(input.args))),
    );

    map.insert(
        "extract_keywords_to_news_keywords".to_string(),
        Arc::new(|input: CommandInput| Box::pin(extract_news_keywords(input.args))),
    );

    map.insert(
        "extract_news_event".to_string(),
        Arc::new(|input: CommandInput| Box::pin(extract_news_event(input.args))),
    );

    map.insert(
        "merge_cross_day_news_events".to_string(),
        Arc::new(|input: CommandInput| Box::pin(merge_cross_day_news_events(input.args))),
    );

    map.insert(
        "fetch_all_news".to_string(),
        Arc::new(|input: CommandInput| Box::pin(fetch_all_news(input.args))),
    );

    map
}
