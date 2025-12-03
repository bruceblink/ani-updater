use common::api::ApiResponse;
use common::api::ItemResult;
use spiders::agedm::fetch_agedm_ani_data;
use spiders::bilibili::fetch_bilibili_ani_data;
use spiders::douban::fetch_douban_movie_data;
use spiders::health_checker::health_check;
use spiders::iqiyi::fetch_iqiyi_ani_data;
use spiders::mikanani::fetch_mikanani_ani_data;
use spiders::news::fetch_latest_news_data;
use spiders::tencent::fetch_qq_ani_data;
use spiders::youku::fetch_youku_ani_data;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

// CmdFn 表示：接收 String 参数（arg/url），返回一个 boxed future，输出为 Result<ApiResponse<AniItemResult>, String>
pub type CmdFn = Arc<
    dyn Fn(String) -> Pin<Box<dyn Future<Output = Result<ApiResponse<ItemResult>, String>> + Send>>
        + Send
        + Sync,
>;

// 示例：构建命令表（把你的实际命令注册进来）
// 注意：把实际的异步命令包装为 `CmdFn`。例如你的 Tauri 命令 `fetch_agedm_ani_data`：
//
// 在这里把它包装为 CmdFn：
pub fn build_cmd_map() -> HashMap<String, CmdFn> {
    let mut map: HashMap<String, CmdFn> = HashMap::new();
    map.insert(
        "fetch_bilibili_ani_data".to_string(),
        Arc::new(|url| Box::pin(fetch_bilibili_ani_data(url))),
    );
    map.insert(
        "fetch_iqiyi_ani_data".to_string(),
        Arc::new(|url| Box::pin(fetch_iqiyi_ani_data(url))),
    );
    map.insert(
        "fetch_mikanani_ani_data".to_string(),
        Arc::new(|url| Box::pin(fetch_mikanani_ani_data(url))),
    );
    map.insert(
        "fetch_qq_ani_data".to_string(),
        Arc::new(|url| Box::pin(fetch_qq_ani_data(url))),
    );
    map.insert(
        "fetch_youku_ani_data".to_string(),
        Arc::new(|url| Box::pin(fetch_youku_ani_data(url))),
    );
    map.insert(
        "fetch_agedm_ani_data".to_string(),
        Arc::new(|url| Box::pin(fetch_agedm_ani_data(url))),
    );

    map.insert(
        "fetch_douban_movie_data".to_string(),
        Arc::new(|url| Box::pin(fetch_douban_movie_data(url))),
    );

    map.insert(
        "fetch_latest_news_data".to_string(),
        Arc::new(|args| Box::pin(fetch_latest_news_data(args))),
    );

    map.insert(
        "health_check".to_string(),
        Arc::new(|urls| Box::pin(health_check(urls))),
    );

    map
}
