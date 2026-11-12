# WatchBridge (development preview)

Windows 上的 Apple Watch 开发者模式准备与 Companion 应用安装助手。

这是基于 [AndreySemjonov 的 iLoader fork](https://github.com/AndreySemjonov/iloader)
的独立社区衍生版本。上游为 [nab138/iLoader](https://github.com/nab138/iloader)。
不代表 Apple 或任何上游项目的官方版本，也不暗示其背书。

## 第一版范围

1. USB 连接已配对的 iPhone，选择设备。
2. 配对 Watch，在手表上确认信任；请求显示开发者模式开关。
3. 在手表上手动开启、重启并确认，然后重新检查状态。
4. 登录 Apple ID，选择包含 iPhone 宿主和 `Watch/*.app` 的原始 IPA，签名安装。
5. 在手表上手动打开应用，验证启动。

开发者模式采用 Rust 原生流程，不要求用户另行安装 Python。准备操作不会自动开启
开发者模式、重启手表或代替用户确认。Watch 信任记录只保存在本程序的内存中，退出
程序后需要重新建立会话。第一版拒绝多块手表的模糊选择。

安装沿用 Andrey fork 的 USB 与 Network/RSD 路径。独立 Watch-only 包暂不支持。
签名之前会检查并保留 IPA，拒绝不包含 Watch Companion 的包。
使用 Network 前需要在 USB 设备卡片上设置 Wi-Fi；开发者模式准备仍需 USB。

## 登录服务

经用户授权，新安装默认使用第三方 `ani.jkcoxson.com` anisette 服务；既有设置不覆盖。
客户端身份与 user agent 从所选服务实时读取，不再写死。2026-11-12 公开接口核查
返回的是 Mac 身份，与教程当时描述不同；这不构成 Windows 登录兼容性证明。
若登录失败，可在设置中选择服务；程序不会自动把数据转到另一个服务。

## 当前验证边界

这是待实机验收的开发预览。上游历史实机成功不等于本分支已经实机通过。
不得把编译成功或手机安装成功当成 Watch 安装、启动成功。
具体测试步骤见 [硬件验收](docs/WATCHBRIDGE_TESTING.md)。

本次修改保留上游签名策略。需要 Watch App Group 的应用可能仍受上游跳过 Watch
App Group 关联行为影响；本版不承诺 HermesEnabler 的备份功能已经修复。

## 构建

Windows 构建依赖和流程见 [BUILDING.md](BUILDING.md)。使用 Windows 原生构建，
不要求 Linux 交叉编译。GitHub Actions 按当前用户要求仅编译、不运行测试，并生成 `WatchBridge.exe`、
`BUILD.json`、`SHA256SUMS.txt` 和 `LICENSES.zip`，其中包含准确源码提交和校验值。

```powershell
./scripts/Build-Windows.ps1 -BuildExecutable -SkipTests
```

仅检查前端：`bun install --frozen-lockfile --ignore-scripts` 后运行 `bun run build`。

## 致谢与许可证

当前依赖链中发现签名库许可声明不明确，详见 [许可核查](docs/LICENSE_REVIEW.md)。
在澄清前，不宣称全依赖许可审核已完成，也不发布正式二进制 Release。


感谢 nab138、AndreySemjonov、Data_C0re_ (Rzbck)、oacevedo、Kabir Oberai、
idevice 与其余上游作者。代码来源、准确提交与许可见
[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)、[LICENSE](LICENSE)、
[vendor/isideload/PROVENANCE.md](vendor/isideload/PROVENANCE.md)。
保留了[上游 README](docs/UPSTREAM_README.md)和上游贡献规则。当前分支是本 fork 的
独立开发工作，不向禁止 AI 生成贡献的上游提交 PR。

请勿提交 Apple ID、密码、证书私钥、配对记录或未脱敏设备日志。
