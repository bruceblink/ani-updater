#!/usr/bin/env bash
set -x
set -eo pipefail

if ! [ -x "$(command -v sqlx)" ]; then
  echo >&2 "Error: sqlx is not installed."
  echo >&2 "Use:"
  echo >&2 "    cargo install --version='~0.8' sqlx-cli --no-default-features --features rustls,postgres"
  echo >&2 "to install it."
  exit 1
fi

# Check if a custom parameter has been set, otherwise use default values
DB_PORT="${DB_PORT:=5432}"
SUPERUSER="${SUPERUSER:=postgres}"
SUPERUSER_PWD="${SUPERUSER_PWD:=password}"
APP_USER="${APP_USER:=app}"
APP_USER_PWD="${APP_USER_PWD:=secret}"
APP_DB_NAME="${APP_DB_NAME:=ani_tracker}"

# Allow to skip Docker if a dockerized Postgres database is already running
if [[ -z "${SKIP_DOCKER}" ]]
then
  # if a postgres container is running, print instructions to kill it and exit
  RUNNING_POSTGRES_CONTAINER=$(docker ps --filter 'name=postgres' --format '{{.ID}}')
  if [[ -n $RUNNING_POSTGRES_CONTAINER ]]; then
    echo >&2 "there is a postgres container already running, kill it with"
    echo >&2 "    docker kill ${RUNNING_POSTGRES_CONTAINER}"
    exit 1
  fi
  CONTAINER_NAME="postgres_$(date '+%s')"
  # Launch postgres using Docker
  docker run \
      --env POSTGRES_USER=${SUPERUSER} \
      --env POSTGRES_PASSWORD=${SUPERUSER_PWD} \
      --health-cmd="pg_isready -U ${SUPERUSER} || exit 1" \
      --health-interval=1s \
      --health-timeout=5s \
      --health-retries=5 \
      --publish "${DB_PORT}":5432 \
      --detach \
      --name "${CONTAINER_NAME}" \
      postgres -N 1000
      # ^ Increased maximum number of connections for testing purposes
      
  until [ \
    "$(docker inspect -f "{{.State.Health.Status}}" ${CONTAINER_NAME})" == \
    "healthy" \
  ]; do     
    >&2 echo "Postgres is still unavailable - sleeping"
    sleep 1 
  done

  # 1. 创建目标数据库 (Superuser)
  CREATE_DB_QUERY="CREATE DATABASE ${APP_DB_NAME} OWNER ${APP_USER};"
  docker exec -it "${CONTAINER_NAME}" psql -U "${SUPERUSER}" -c "${CREATE_DB_QUERY}"

  # 2. 创建应用程序用户 (Superuser)
  CREATE_USER_QUERY="CREATE USER ${APP_USER} WITH PASSWORD '${APP_USER_PWD}';"
  docker exec -it "${CONTAINER_NAME}" psql -U "${SUPERUSER}" -c "${CREATE_USER_QUERY}"

  # 3. **新增步骤：安装扩展 (Superuser)**
  #    连接到 APP_DB_NAME 数据库，并安装 vector 扩展。
  INSTALL_EXTENSION_QUERY="CREATE EXTENSION IF NOT EXISTS vector;"
  docker exec -it "${CONTAINER_NAME}" psql -U "${SUPERUSER}" -d "${APP_DB_NAME}" -c "${INSTALL_EXTENSION_QUERY}"

  # 4. 授予/撤销权限
  # 由于前面已经创建了数据库并指定了 Owner 为 APP_USER，
  # 应用程序用户现在对该数据库拥有完全控制权，通常不再需要 'CREATEDB' 权限。
  # 因此，为了遵循最小权限原则，我建议移除之前的 GRANT_QUERY。
  # 如果您仍然需要 CREATEDB 权限，请保持原样。

fi

>&2 echo "Postgres is up and running on port ${DB_PORT} - running migrations now!"

# 使用 APP_USER 的凭证连接
DATABASE_URL=postgres://${APP_USER}:${APP_USER_PWD}@localhost:${DB_PORT}/${APP_DB_NAME}
export DATABASE_URL
sqlx database create # 这一步现在是可选的，或者会检查数据库是否已存在
sqlx migrate run # 现在扩展已安装，迁移将顺利通过

>&2 echo "Postgres has been migrated, ready to go!"
