-- Add migration script here
insert into scheduled_tasks (name, cron, params, is_enabled, last_run, next_run, last_status) values ('哔哩哔哩国创', '0 17 10,11,12,17,18,19,20,21,22,23 * * * *', '{
  "arg": "https://api.bilibili.com/pgc/web/timeline?types=4&before=6&after=6",
  "cmd": "fetch_bilibili_ani_data"
}', true,  null, null, '0');
insert into scheduled_tasks (name, cron, params, is_enabled, last_run, next_run, last_status) values ('哔哩哔哩番剧', '0 17 10,11,12,17,18,19,20,21,22,23 * * * *', '{
  "arg": "https://api.bilibili.com/pgc/web/timeline?types=1&before=6&after=6",
  "cmd": "fetch_bilibili_ani_data"
}
', true,  null, null, '0');
insert into scheduled_tasks (name, cron, params, is_enabled, last_run, next_run, last_status) values ('agedm', '0 17 10,11,12,17,18,19,20,21,22,23 * * * *', '{
  "arg": "https://www.agedm.io/update",
  "cmd": "fetch_agedm_ani_data"
}
', true,  null, null, '0');
insert into scheduled_tasks (name, cron, params, is_enabled, last_run, next_run, last_status) values ('爱奇艺动漫', '0 17 10,11,12,17,18,19,20,21,22,23 * * * *', '{
  "arg": "https://mesh.if.iqiyi.com/portal/lw/v7/channel/cartoon",
  "cmd": "fetch_iqiyi_ani_data"
}
', true,  null, null, '0');
insert into scheduled_tasks (name, cron, params, is_enabled, last_run, next_run, last_status) values ('腾讯动漫', '0 17 10,11,12,17,18,19,20,21,22,23 * * * *', '{
  "arg": "https://v.qq.com/channel/cartoon",
  "cmd": "fetch_qq_ani_data"
}
', true,  null, null, '0');
insert into scheduled_tasks (name, cron, params, is_enabled, last_run, next_run, last_status) values ('优酷动漫', '0 17 10,11,12,17,18,19,20,21,22,23 * * * *', '{
  "arg": "https://www.youku.com/ku/webcomic",
  "cmd": "fetch_youku_ani_data"
}
', true,  null, null, '0');
insert into scheduled_tasks (name, cron, params, is_enabled, last_run, next_run, last_status) values ('豆瓣电影-热门', '0 17 10,11,12,17,18,19,20,21,22,23 * * * *', '{
  "arg": "https://m.douban.com/rexxar/api/v2/subject/recent_hot/movie?category=%E7%83%AD%E9%97%A8&type=%E5%85%A8%E9%83%A8&ck=sLqV",
  "cmd": "fetch_douban_movie_data"
}
', true,  null, null, '0');
insert into scheduled_tasks (name, cron, params, is_enabled, last_run, next_run, last_status) values ('豆瓣电影-最新', '0 17 10,11,12,17,18,19,20,21,22,23 * * * *', '{
  "arg": "https://m.douban.com/rexxar/api/v2/subject/recent_hot/movie?category=%E6%9C%80%E6%96%B0&type=%E5%85%A8%E9%83%A8&ck=sLqV",
  "cmd": "fetch_douban_movie_data"
}
', true,  null, null, '0');
insert into scheduled_tasks (name, cron, params, is_enabled, last_run, next_run, last_status) values ('最新新闻', '0 */20 6-23,0-1 * * * *', '{
  "arg": "v2ex,v2ex-share,zhihu,weibo,zaobao,cls-hot,coolapk,mktnews,mktnews-flash,wallstreetcn,wallstreetcn-quick,wallstreetcn-news,wallstreetcn-hot,36kr,36kr-quick,douyin,hupu,tieba,toutiao,ithome,douban,thepaper,sputniknewscn,cankaoxiaoxi,pcbeta,pcbeta-windows11,cls,cls-telegraph,cls-depth,cls-hot,xueqiu,xueqiu-hotstock,gelonghui,fastbull,fastbull-express,fastbull-news,solidot,hackernews,producthunt,github,github-trending-today,bilibili,bilibili-hot-search,bilibili-hot-video,bilibili-ranking,kuaishou,kaopu,jin10,baidu,nowcoder,sspai,juejin,ifeng,chongbuluo,chongbuluo-latest,chongbuluo-hot,steam,tencent-hot",
  "cmd": "fetch_latest_news_data"
}
', true,  null, null, '0');
