# V2Ray/Sing-Box Subscription Converter

一个轻量、高效的订阅转换工具，能够将 V2Ray (vmess / vless / ss / trojan) 订阅链接转换为 sing-box 适用的 JSON 配置文件，并支持按国家/地区/策略组进行灵活的分流过滤。

---

## 🚀 特性

- **多协议支持**：支持解析 `vmess://`、`vless://`、`ss://`、`trojan://` 节点。
- **自定义模板**：支持挂载本地 `template.yaml` 模版，生成完全贴合个人习惯的规则与出站配置。
- **Docker 一键部署**：提供 Docker 和 Docker Compose 快速运行方案。
- **无隐私泄露**：完全运行在您自己的本地环境或服务器中，没有任何中间服务器记录您的订阅信息。

---

## 🛠️ 构建与发布 (Docker Hub)

如果您希望自行构建镜像并推送到 Docker Hub (docker.io)：

### 1. 本地构建与测试
在项目根目录下执行：
```bash
docker build -t <your-dockerhub-username>/v2ray-singbox:latest .
```

### 2. 登录 Docker Hub
```bash
docker login
```

### 3. 推送到 Docker Hub
```bash
docker push <your-dockerhub-username>/v2ray-singbox:latest
```

### 4. 通过 GitHub Actions 自动构建与推送 (推荐)
项目已内置 GitHub Actions 自动化流水线（位于 `.github/workflows/docker-publish.yml`）。每当您将代码推送（push）到 `master` 分支时，将自动触发构建并推送镜像至您的 Docker Hub 仓库。

#### 配置步骤：
1. **获取 Docker Hub 访问令牌 (Token)**:
   - 登录 Docker Hub，前往 **Account Settings** -> **Security** -> **Personal Access Tokens**，新建一个 Access Token（建议给予 Read & Write 权限）并复制保存。
2. **在 GitHub 仓库配置 Secrets**:
   - 打开您的 GitHub 项目仓库，前往 **Settings** -> **Secrets and variables** -> **Actions**。
   - 点击 **New repository secret**，添加以下两个 Secrets：
     - `DOCKER_USERNAME`：您的 Docker Hub 用户名。
     - `DOCKER_PASSWORD`：刚刚在 Docker Hub 生成的访问令牌 (Token)。
3. **触发构建**:
   - 只要有代码 push 到 `master` 分支，或在 GitHub 项目的 **Actions** 菜单中手动点击 **Run workflow**，系统便会全自动构建并发布最新镜像。

---

## 📦 部署指南 (Deployment)

### 方法一：使用 Docker Compose (推荐，支持自定义模版一键部署)

1. 创建一个专属目录（例如 `v2ray-singbox`）并在该目录下放置您的 `template.yaml` 文件。
2. 创建 `docker-compose.yml` 文件，写入以下内容：

```yaml
services:
  v2ray-singbox:
    image: <your-dockerhub-username>/v2ray-singbox:latest  # 或直接在本地使用 build: .
    container_name: v2ray-singbox
    ports:
      - "3000:3000"
    environment:
      - PORT=3000
    volumes:
      - ./template.yaml:/app/template.yaml
    restart: unless-stopped
```

3. 启动服务：
```bash
docker compose up -d
```

### 方法二：使用 Docker CLI 运行

如果您不需要外置挂载模版文件，或者想直接运行：

```bash
docker run -d \
  --name v2ray-singbox \
  -p 3000:3000 \
  -e PORT=3000 \
  <your-dockerhub-username>/v2ray-singbox:latest
```

如需外置挂载本地 `template.yaml`：

```bash
docker run -d \
  --name v2ray-singbox \
  -p 3000:3000 \
  -e PORT=3000 \
  -v $(pwd)/template.yaml:/app/template.yaml \
  <your-dockerhub-username>/v2ray-singbox:latest
```

---

## 📝 接口使用说明 (API Usage)

服务启动后，监听端口为 `3000`（可在环境变量 `PORT` 中修改）。

### 转换接口

- **路径**: `/sub`
- **方法**: `GET`
- **请求参数**:
  - `url` (必填): 您的原始订阅链接（需要进行 URL 编码）。

#### 请求示例：
```text
http://localhost:3000/sub?url=https%3A%2F%2Fexample.com%2Fpath%2Fto%2Fsub
```

#### 返回结果：
返回转换合并后的 `sing-box` 格式 JSON 配置文件。

---

## 🔒 隐私与安全

1. **零数据留存**：此工具仅作为无状态的转换代理，在内存中完成获取、解析、组装并直接返回，不会写入任何本地数据库或临时文件。
2. **纯净构建**：`.dockerignore` 文件已配置排除了本地构建目录 `target`、编译缓存以及敏感环境配置，确保构建生成的镜像不包含任何本地隐私数据。
