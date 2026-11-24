# QingLink 工作区规则（AGENTS.md）

> DSH 会按 `$DSH_HOME/AGENTS.md`（全局基线）→ 项目根目录 → 会话工作目录的顺序逐级加载
> `AGENTS.md` / `CLAUDE.md`（含 `.local` 覆盖层），每次组装请求时重读，改完本文件立即生效，
> 无需重启会话；子目录下的 `AGENTS.md` 在触达该目录时生效。
>
> 本文件是**工程约束的唯一权威来源**；[`.opencode/rules/`](.opencode/rules) 是分语言的编码规范细节
> （Rust / Dart / 架构）。两者冲突时，按**更严格**的一条执行。

适用对象：所有在本仓库工作的 agent（DSH、opencode 等）与人类贡献者。

---

## 1. 沟通与注释语言

- 与用户的对话、评审结论、提交说明：**中文**。
- 代码注释、公开 API 文档与 UI 文案：**中文**（`rust-coding.md` §8.3、`dart-coding.md` §8.2）。

---

## 2. 代码不得存在大文件（硬约束）

**大文件不是风格问题，是缺陷。** 一个文件承载多个职责会导致评审、合并与重构成本持续上升；
本仓库不接受"先把功能写完，以后再拆"。

### 2.1 行数上限

| 对象 | 建议值 | 硬上限 | 处理 |
| --- | --- | --- | --- |
| 源码单文件（`.rs` / `.dart` / `.ts`） | ≤ 400 行 | **500 行** | 超过硬上限必须拆分 |
| 单文件 Vue 组件（`.vue`） | ≤ 300 行 | **400 行** | 逻辑抽到 composable / store |
| 单个函数 / 方法 | ≤ 80 行 | **120 行** | 提取子函数，禁止用折叠注释掩盖 |
| 单个 struct / class / Widget 类 | ≤ 300 行 | 400 行 | 按职责拆分类型 |
| 文档单篇（`docs/*.md`、`.opencode/rules/*.md`） | ≤ 800 行 | 1200 行 | 按主题拆分成多篇并更新 `README.md` 的「文档」表 |

- "行数"指文件的**物理总行数**：含空行与注释。口径以 `wc -l` / `(Get-Content <file>).Count`
  为准，文件末尾的换行符不额外算一行（本文件 §2.3 表内的数字就是按此口径统计的）。
- 上限值就是上表。本仓库**没有行数校验脚本**，构建与校验一律用语言原生命令（见 §7）；
  改上限必须同时改上表、两份编码规范的 §11 与 [`agent.md`](agent.md) 的审查清单。
- 测试目录（`tests/`、`test/`、`*_test.*`）与生成目录不纳入统计（本仓库不新增测试代码，见 §4）。
- 现有 `docs/web-api.md`（1195 行）超出建议值、未超硬上限。**按主题拆分已被评估并推迟**：
  该文档的 `§4.x` 编号被前端 `apps/desktop/web-ui/src/api/*.ts` 的注释与文档内部大量互引依赖，
  拆分需同时重编所有引用；要做就单独一个改动完成编号迁移，不要顺手拆。行数只能靠
  "删掉与 `protocol/v2.md` 重复的整节"来降，不要再往里加内容。

### 2.2 拆分要求

拆分必须按**职责**切，而不是按行数机械切：

1. 按职责拆模块：`app_scanner.rs` 的"扫描调度"与"路径解析"拆成两个模块，而不是 `app_scanner_part2.rs`。
2. 共享状态抽成独立类型，用依赖注入传入，而不是靠 `pub(crate)` 暴露内部字段。
3. 拆出的模块必须遵守 crate 依赖方向（见 `.opencode/rules/architecture.md` §1）。
4. 公开 API 尽量保持兼容；必须变更时同步更新文档（见 §3）。
5. 禁止用以下方式"规避"行数统计，一经发现按未完成处理：
   - 压缩语句、把多行合并成超长行、删除必要空行；
   - 把代码挪进 `#[cfg(...)]`、注释块或字符串；
   - 用 `include!` / 动态拼装把代码藏在别处。

### 2.3 既有超限文件清单（只许减少）

当前仍有 **3 个**文件超出所属类型上限。**这份清单就是棘轮**：清单内的文件只允许变短。

| 文件 | 当前行数 | 上限 |
| --- | --- | --- |
| `apps/mobile/lib/src/screens/scan_folder_screen.dart` | 659 | 500 |
| `apps/mobile/lib/src/screens/pairing_screen.dart` | 650 | 500 |
| `crates/qinglink-server/src/protocol/requests_ws.rs` | 535 | 500 |

行数按 §2.1 的口径统计：`tests/`、`*_test.*` 与生成目录不计入。

规则：

- **只减不增**：清单内文件的行数不得超过表中数字，也不得持平后继续追加内容。
- **改完必须同步本表**：行数发生变化时，在同一次改动里把表中数字改成新的实际值（§3 的要求）。
- **降回所属类型上限以内即移出清单**，不得继续留在表中。
- **新增文件一律受 §2.1 的硬上限约束，没有豁免。**
- 修改清单内文件时**优先顺手拆分**；无法拆分的小改动允许，但总行数必须不增加。
- **不设"登记新债"通道**：新增超限文件一律按未完成处理。

### 2.4 生成与自查（本仓库没有校验脚本）

本仓库不提供行数校验器。约束靠三件事生效：规则每次组装请求都会被重读（见文首说明）、
判据随时可查（下）、评审按 [`agent.md`](agent.md) 的清单把关。

写代码时：

1. **新建文件前先估算行数**；接近或超过 §2.1 的硬上限就先按职责拆（拆法见 §2.2）。
2. **改既有文件前先查 §2.3 的清单**；在清单里的，改完行数必须 ≤ 表中数字，并同步更新表中数字。
3. 自查用原生命令，不写脚本：

   ```powershell
   # 单个文件的行数
   (Get-Content <file>).Count

   # 全仓库超限文件（.vue 用 400，其余用 500）
   Get-ChildItem -Recurse -Include *.rs,*.dart,*.ts,*.vue |
     Where-Object { $_.FullName -notmatch 'node_modules|[\\/]target[\\/]|[\\/]build[\\/]|[\\/]dist[\\/]|[\\/]tests?[\\/]|[._](test|spec)\.[a-z]+$|ephemeral_deliverables|\.plugin_symlinks' } |
     ForEach-Object {
       $limit = if ($_.Extension -eq '.vue') { 400 } else { 500 }
       $lines = (Get-Content $_).Count
       if ($lines -gt $limit) { [pscustomobject]@{ Lines = $lines; Limit = $limit; Path = $_.FullName } }
     } | Sort-Object Lines -Descending
   ```

4. **函数 / 类型粒度**（120 / 400 行）没有机器判据，由生成时的自觉与评审把关。

---

## 3. 文档必须跟随项目同步

**代码与文档在同一次改动中提交。** 禁止"先改代码，文档以后补"——本仓库的文档是契约，
不是事后说明；文档落后于实现等于给出错误的接口描述。

### 3.1 触发映射（改了什么 → 必须同步什么）

| 改动位置 | 必须同步的文档 |
| --- | --- |
| `crates/qinglink-protocol/**`、`protocol/**` | [`protocol/v2.md`](protocol/v2.md)、[`protocol/requests.md`](protocol/requests.md)、[`protocol/README.md`](protocol/README.md)、`protocol/qinglink-v2.schema.json`、`protocol/qinglink-v2-ws.schema.json`、`protocol/fixtures/*` |
| `crates/qinglink-server/src/server/**`（局域网 `/api/v2/*`） | [`protocol/v2.md`](protocol/v2.md)；**新增 / 删除 / 改名字段或错误码**时，同批更新 [`protocol/requests.md`](protocol/requests.md) 的对应行 |
| 桌面管理接口（`services/qinglink-service` 管理面、`apps/desktop/src/web/**`） | [`docs/web-api.md`](docs/web-api.md)、`apps/desktop/web-ui/src/api/contracts/*.ts` |
| crate 划分、依赖方向、模块职责、线程模型 | [`docs/architecture.md`](docs/architecture.md)、[`.opencode/rules/architecture.md`](.opencode/rules/architecture.md)、[`README.md`](README.md) 结构段 |
| 构建方式、运行参数、端口、数据目录、排障 | [`docs/development.md`](docs/development.md)、[`README.md`](README.md) |
| `apps/desktop/build.rs` 的前端 / 资源嵌入逻辑 | [`docs/development.md`](docs/development.md) §2.1、本文件 §7 |
| `crates/qinglink-version/**`、版本号构成与递增规则 | [`docs/development.md`](docs/development.md) §2.4、[`protocol/README.md`](protocol/README.md) |
| 移动端行为、配对流程 | [`apps/mobile/README.md`](apps/mobile/README.md)、[`docs/architecture.md`](docs/architecture.md) 相关时序 |
| Rust / Dart 编码约定本身 | [`.opencode/rules/rust-coding.md`](.opencode/rules/rust-coding.md)、[`.opencode/rules/dart-coding.md`](.opencode/rules/dart-coding.md)（评审口径见 [`agent.md`](agent.md)） |
| 新增 / 删除 / 重命名文档 | [`README.md`](README.md) 的「文档」表 |
| 本文件规定的内容 | `AGENTS.md`（本文件）自身 |

### 3.2 什么才算"同步"

- 字段、错误码、端点、消息类型、默认值：**逐项落到对应表格**，不允许只追加一句
  "新增支持 X"。
- 文档中引用的路径、文件名、命令必须真实存在且可执行。
- 功能删除时，文档同步删除；不允许保留"历史遗留 / 已废弃"描述来掩盖不一致。
- 文档中的示例代码必须与当前实现一致（复制粘贴可编译/可运行）。
- 行数上限、清单条目数等**数字型描述**随改动更新（§2.1、§2.3 两张表与本文件附录保持一致）。

### 3.3 判定与后果

- 评审时发现"代码已改、文档未改"：按 [`agent.md`](agent.md) 的 **HIGH**（接口契约类为 **BLOCKER**）处理。
- 无法判断应更新哪份文档时，先查 §3.1；仍无对应项，则在改动说明中明确写出"本改动不影响任何文档"。

---

## 4. 测试策略

- **本仓库不需要新增测试代码。** 不生成 `#[test]` / `#[cfg(test)]`、`*_test.rs`、`*_test.dart`、
  `tests/` 下的新文件，除非人类在当次任务中明确要求。
- **不运行测试套件。** 不执行 `cargo test`、`flutter test`、`npm test`。
  `cargo check --workspace --all-targets` 属于静态编译检查（只编译、不运行）允许执行。
- 既有测试文件保留；本规则不授权删除它们。
- 验证改用静态手段：

  ```powershell
  cargo fmt --all --check                              # Rust 格式
  cargo check --workspace --all-targets                # Rust 编译检查
  cd apps\desktop\web-ui; npm run typecheck            # 前端类型检查
  cd apps\mobile; flutter analyze                      # Dart 静态检查
  ```

- 限制的成因是既有约定（`agent.md` 早已写明"不需要 test 代码"）；若需要改变该策略，
  由人类明确指示后再改本文件。

---

## 5. 既有规范索引

| 文件 | 内容 |
| --- | --- |
| [`.opencode/rules/architecture.md`](.opencode/rules/architecture.md) | crate 依赖图、模块职责、协议契约、桌面端线程模型、禁止模式 |
| [`.opencode/rules/rust-coding.md`](.opencode/rules/rust-coding.md) | Rust 命名、Import、类型、错误处理、unsafe、异步、文档 |
| [`.opencode/rules/dart-coding.md`](.opencode/rules/dart-coding.md) | Dart 命名、类型安全、Riverpod、Widget 模式、常量 |
| [`agent.md`](agent.md) | 代码审查口径：判定项、严重级别定级表、输出格式与升级条件（语言级细则不重复） |
| [`docs/architecture.md`](docs/architecture.md) | 仓库级结构与设计取舍 |
| [`docs/web-api.md`](docs/web-api.md) | 桌面管理接口契约 |
| [`docs/development.md`](docs/development.md) | 构建、运行、自检、排障、改动约定 |

---

## 6. 完成定义（Definition of Done）

一次改动只有全部满足以下条件才能声称完成：

- [ ] 新增 / 修改的文件符合 §2.1 的上限；§2.3 清单内文件未被加长，且表中数字已按本次改动更新。
- [ ] §3.1 映射到的每一份文档都已在同一次改动中更新。
- [ ] 未新增测试代码，未运行测试套件。
- [ ] 未新增任何脚本（§7）。
- [ ] 静态检查按改动范围执行（`cargo fmt --all --check`、`cargo check --workspace --all-targets`、`npm run typecheck`、`flutter analyze`）。
- [ ] 没有用 §2.2 列举的手段规避行数统计。

**明确禁止的行为**（一律判为未完成）：

1. 靠空行、注释、超长行、`#[cfg]` 等手段把文件"压"到上限以内。
2. 把代码机械切成 `*_part2.rs` / `*_2.dart` 之类的碎片文件。
3. 只在文档末尾补一句"支持 X"，而不更新对应表格与会话流程。
4. 把 §2.3 清单里的行数写大，或把已降回上限以内的文件继续留在清单里。
5. 新增测试代码、运行测试套件，或在报告里声称"测试通过"。
6. 未逐条对照 §2 / §3 / §7 的判据就声称任务完成。

---

## 7. 构建与校验：不设脚手架脚本

跨语言编排由 [`apps/desktop/build.rs`](apps/desktop/build.rs) 承担——它相当于 Tauri 的
`tauri.conf.json` 加 CLI：`web-ui/dist` 缺失或过旧时自动构建前端并内嵌，服务二进制按约定
从 `apps/desktop/assets/input/` 或 `target/{debug,release}/` 取用。因此本项目**不需要**任何
包装 `cargo` / `npm` / `flutter` 的脚本。

版本资源的生成也属于构建脚本：`crates/qinglink-version/build.rs` 维护构建号，
`apps/desktop/build.rs` 与 `services/qinglink-service/build.rs` 按当前构建号生成
`VERSIONINFO` 并嵌入（规则见 [`docs/development.md`](docs/development.md) §2.4）。

- 构建、运行、校验、发布一律用语言原生命令：`cargo`、`npm`、`flutter`。
- **禁止新增包装脚本**（PowerShell / shell / Node / `cargo` 子命令都一样），也禁止恢复历史上
  存在过的 `scripts/*.ps1` 与行数校验器。
- 前端工程自带的 npm 命令（`dev` / `build` / `build:fallback` / `typecheck`）属于 Node 工具链
  的命令面，`build.rs` 直接调用，不受此限制。
- 真正需要多步顺序的流程（服务资源刷新、发布打包）写在
  [`docs/development.md`](docs/development.md) 的命令块里，由 `build.rs` 的两条
  `cargo:warning` 守卫兜底：服务资源旧于服务源码、内嵌界面旧于前端源码。
- 判据与自查方式见 §2.4；模型或贡献者新增"方便一下"的脚本一律按未完成处理。

---

## 附：当前清单摘要

| 指标 | 值 |
| --- | --- |
| 硬上限 / 警戒线 | 500 行（`.vue` 400）/ 400 行 |
| §2.3 清单内的超限文件 | 3 |
| 清单内最大文件 | `apps/mobile/lib/src/screens/scan_folder_screen.dart`（659 行） |

§2.3 的表格是权威数据；本表是快照，拆分或新增后应同步更新这两行（§3.2）。
