# 短信智标官（SMS Tagging Officer）

一款离线桌面工具：导入短信（CSV/XLSX）→ 批量打标（规则优先 + 可选模型补全）→ 人工复核 → 导出（CSV/JSONL）。

- 前端：Vue 3 + Vite + TypeScript
- 桌面壳：Tauri（Rust）
- 数据库：SQLite（内置、落在系统的应用数据目录）

> 说明：当前“模型推理”默认是 `mock`（不跑模型，仅规则/灰区需复核）。如果要离线跑 GGUF 模型，可配置 `llama_cli` Provider：本项目通过外部 `llama-cli` 可执行文件调用（便于离线、可替换），并非 FFI 方式把 llama.cpp 静态内嵌进进程。

---

## 1. 功能概览

- **导入**：CSV / XLSX 预览与列映射（至少要有 `content` 列）。
- **规则引擎（优先）**：验证码/物流取件/政务通知/金融交易提醒等强命中直接出结果，并抽取实体（URL/金额/验证码/尾号等）。
- **模型 Provider（可选）**：灰区短信可调用 `llama-cli`（GGUF）按严格 JSON 输出格式补全标签与实体。
- **融合策略**：规则强命中优先；规则与模型冲突时 `needs_review=true` 并降低置信度。
- **批处理队列**：并发 worker、超时、重试、失败 ID 可重试、错误落盘、进度事件推送前端。
- **列表/筛选/复核**：按行业/类型/needs_review/置信度/是否含 URL/金额/验证码等筛选；抽屉编辑并写入审计日志。
- **导出**：
  - `JSONL`：每行一个 `LabelOutput` JSON
  - `CSV`：把实体字段展开成列

---

## 2. 标签枚举（固定）

一级行业（6）：`金融 / 通用 / 政务 / 渠道 / 互联网 / 其他`

二级短信类型（10）：
`验证码 / 交易提醒 / 账单催缴 / 保险续保 / 物流取件 / 会员账号变更 / 政务通知 / 风险提示 / 营销推广 / 其他`

---

## 3. 输出 JSON（稳定结构）

核心结构见 [src-tauri/src/model/schema.rs](src-tauri/src/model/schema.rs)。

- `industry`：行业枚举
- `type`：短信类型枚举（字段名为 `type`，Rust 内部为 `sms_type`）
- `entities`：必须包含：
  - `brand`, `verification_code`, `amount`, `balance`, `account_suffix`, `time_text`, `url`, `phone_in_text`
  - 缺失填 `null`
- `confidence`：`0~1`
- `needs_review`：是否需要人工复核
- `reasons`：可解释原因数组（字符串）
- `signals`：规则/特征信号（用于解释与调试）
- `rules_version` / `model_version` / `schema_version`

---

## 4. 环境准备（macOS）

- Node.js 18+（建议 20+）
- Rust stable（建议 1.75+）
- Xcode Command Line Tools：`xcode-select --install`

安装依赖：

```bash
npm install
```

后端编译检查：

```bash
cargo check --manifest-path src-tauri/Cargo.toml
```

---

## 5. 开发运行（Tauri Dev）

```bash
npm run tauri -- dev
```

首次运行会编译 Rust 依赖，时间较长属于正常现象。

---

## 6. 使用流程（导入 → 批处理 → 复核 → 导出）

1) 打开 **导入** 页：选择 CSV/XLSX
- 映射 `content` 列（必填）
- 其他列可选：`received_at / sender / phone / source`

导入示例：
- 英文表头示例：`samples/sms_samples.csv`（自测也用它）
- 中文表头示例（20条）：`samples/top20_import_example.csv`
  - 在导入页把 `content` 映射到 `短信内容`

2) **批处理** 页：
- `mode`：
  - `unlabeled`（默认）：仅处理未打标
  - `needs_review`：仅处理需要复核的
  - `all`：全量
- `concurrency`：并发（1~8）
- `timeout_ms`：单条推理超时（毫秒）
- `max_retries`：失败重试次数

3) **列表复核** 页：筛选 `needs_review=true` 的记录，打开抽屉手工修正并保存。

4) **导出** 页：
- 选择 `JSONL` 或 `CSV`
- `only_reviewed=true` 会过滤掉 `needs_review=true` 的记录

---

## 7. 离线模型（llama-cli / GGUF）配置

默认 Provider：`mock`（不跑模型，仅规则 + 灰区需复核）。

如需离线跑 GGUF：

1) 准备 `llama-cli` 可执行文件（你自行从 llama.cpp 构建/获取）。
2) 在 **设置** 页：
- Provider kind 选择 `llama_cli`
- `model_path`：指向你的 `*.gguf` 文件
- `llama_cli_path`：指向 `llama-cli`（可选）
  - 不填时默认查找：`src-tauri/resources/llama-cli`

3) 点 **Health Check**：
- `ok=true` 才算配置成功

macOS 提示权限时可执行：

```bash
chmod +x /path/to/llama-cli
```

---

## 8. 一键自测（无需模型）

项目自带样例短信：[samples/sms_samples.csv](samples/sms_samples.csv)

运行：

```bash
chmod +x tools/selftest.sh
./tools/selftest.sh
```

自测会：
- 创建/覆盖 `tools/selftest.sqlite3`
- 导入样例短信
- 规则+mock provider 生成标签写入 labels
- 导出：
  - `tools/selftest_export.jsonl`
  - `tools/selftest_export.csv`

---

## 9. 常见问题

- **`tauri dev` 很慢**：首次会下载/编译 Rust 依赖，正常。
- **`llama-cli not found`**：在设置页填 `llama_cli_path`，或把文件放到 `src-tauri/resources/llama-cli`。
- **模型输出不稳定**：本项目会从输出中抽取第一个 JSON 对象并做 `normalize()`；仍建议使用更强的 JSON 约束 prompt 与更低温度。

---

## 10. 目录速览

- 前端：`src/`
- 后端：`src-tauri/src/`
- 迁移：`src-tauri/src/db/migrations/001_init.sql`
- Provider：`src-tauri/src/model/provider.rs`
- 规则：`src-tauri/src/rules/mod.rs`
- 批处理：`src-tauri/src/model/batch.rs`
- 自测：`src-tauri/src/bin/selftest.rs` + `tools/selftest.sh`
