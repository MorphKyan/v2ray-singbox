# ShellCrash 配置备份与恢复指南

这份文档详细说明了在重新安装 ShellCrash 之后，如何使用本地备份的文件恢复所有自定义配置和内存优化补丁。

## 备份的文件列表

在当前目录下（`d:\v2ray-singbox`），已成功为您备份了以下两个核心文件：

1. **`ShellCrash.cfg`**
   - **作用**：ShellCrash 的核心设置面板参数（保存了您的混淆模式、防火墙设置、自定义的 Rust 服务订阅链接等）。
   - **路由器路径**：`/etc/ShellCrash/configs/ShellCrash.cfg`
   
2. **`others.json`**
   - **作用**：针对 Sing-box 内核的内存优化补丁（由于开启了 `cache_file` 磁盘缓存功能，可以防止路由器因解析大体积 `.srs` 规则集而耗尽内存）。
   - **路由器路径**：`/etc/ShellCrash/jsons/others.json`

---

## 恢复配置详细步骤

当您完全卸载并重新安装 ShellCrash 之后，请严格按照以下步骤恢复：

### 第 1 步：重新安装 ShellCrash
按照 ShellCrash 的正常流程将其重新安装到您的路由器中。
**注意**：安装完成并下载好内核后，**先不要**在管理菜单里按 `1` 启动服务。

### 第 2 步：将备份文件覆盖回路由器
您可以直接使用 `pscp` 命令行工具（与下载时一样），在 `d:\v2ray-singbox` 目录下打开 PowerShell，执行以下两行命令：

1. 覆盖主配置文件：
```powershell
echo y | pscp.exe -scp -pw home14259598 ShellCrash.cfg root@192.168.1.1:/etc/ShellCrash/configs/ShellCrash.cfg
```

2. 覆盖内存优化补丁文件：
```powershell
echo y | pscp.exe -scp -pw home14259598 others.json root@192.168.1.1:/etc/ShellCrash/jsons/others.json
```

*(如果您习惯使用 WinSCP 等图形化 SFTP 工具，直接将这两个文件分别拖入上述对应的路由器路径覆盖即可。)*

### 第 3 步：启动并验证
1. SSH 连接到路由器，输入 `crash` 唤出控制面板。
2. 随便浏览一下各项设置，确认您之前的配置（比如混合模式、订阅地址）都已经出现。
3. 按 `1` 启动 ShellCrash 服务。
4. 启动完成后，Sing-box 就会自动加载 `others.json` 里的补丁配置，重新在后台生成 `cache.db` 缓存数据库文件，此时您的优化环境即宣告 100% 恢复成功！
