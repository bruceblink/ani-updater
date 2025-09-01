# Github第三方登录使用说明
## 环境准备
注册GitHub账号并创建OAuth应用
- 登录GitHub，进入Settings -> Developer settings -> OAuth Apps，点击"New OAuth App"。
- 填写应用名称、主页URL（如http://localhost:8000）和回调URL（如http://localhost:8000/auth/github/callback）。
- 创建应用后，记下Client ID和Client Secret。
- 配置应用的权限和回调URL，确保与代码中的配置一致。
- 将Client ID和Client Secret添加到配置文件或环境变量中。

具体的如我下图所示配置
![github](GitHub%20OAuth2.png)

-----
我在项目是使用的是环境变量的方式配置的
需要配置以下环境变量
```bash
GITHUB_CLIENT_ID=${your_client_id} # 从GitHub OAuth应用获取
GITHUB_CLIENT_SECRET=${your_client_secret} # 从GitHub OAuth应用获取
OAUTH_BASE_URL=${HomePage URL} # 后端应用服务的URL
FRONTEND_URL=http://localhost:3039 # 登录成功后跳转的前端应用的URL
FRONTEND_DOMAINS=localhost:3039;example.com # 允许跨域访问后端资源的前端域名白名单列表,分号分隔
```
具体的环境变量配置可以查看[`oauth_config.ts`](../ani_subs/src/routes/auth/oauth_config.rs)文件
配套的前端测试项目可以使用[ani_updater_frontend_test](https://github.com/bruceblink/material-kit-react)