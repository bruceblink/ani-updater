FROM rust:1.86.0
WORKDIR /app
RUN apt update && apt install lld clang -y
COPY . .
# 使用离线模式
ENV SQLX_OFFINE true
# 构建二进制文件 使用release参数优化
RUN cargo build --release
# 运行环境参数
ENV APP_ENVIRONMENT production
# 执行docker run时，启动二进制文件
ENTRYPOINT ["./target/release/ani-updater"]
