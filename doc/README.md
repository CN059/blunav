**项目概览**

本仓库为基于 Java 开源项目 "IndoorPos" 的迁移说明与文档（目标平台：Rust），用于记录系统架构、关键名词、算法实现要点、数据交互格式以及从 Java 向 Rust 迁移时的设计和工程注意事项。本 `README` 旨在为工程师提供快速理解系统、实现算法与完成迁移改造的参考。

**背景说明**：原项目基于 iBeacon（蓝牙 4.0）实现室内定位服务器端，包含三种定位算法：三边定位（Trilateration）、加权三边定位（Weighted Trilateration）和加权三角形质心定位（Weighted Triangle Centroid）。原实现采用 Spring 框架、Netty、JDBC+Druid、RMI 等 Java 技术栈。迁移到 Rust 的目标是提高性能、安全性、并发能力与可维护性，同时保留原有业务与算法逻辑。

**目录**

- **项目概览**: 本页
- **系统架构**: 组件与数据流
- **名词解释**: 相关术语
- **算法实现细节**: RSSI 到距离建模、三种定位算法说明与伪流程
- **数据模型与消息格式**: 数据库与 API 示例
- **迁移说明（Java → Rust）**: 核心组件对照与建议库
- **运行与配置**: 本地运行、环境变量、常用命令
- **测试与验证**: 验证方法和建议用例
- **性能与部署建议**
- **后续工作建议**

**系统架构**

系统主要由五个部分组成：基站（Beacon）、被定位终端（Device/Client）、服务器（Server）、数据库（DB）与显示客户端（Client）。

简化的架构流程（编号对应下文注释）：

```
  (1) Beacon -> broadcasts (beacon_id, power)
  (2) Device -> collects RSSI list from multiple Beacons and sends payload to Server
  (3) Server <- reads beacon coordinates / params from DB; computes location
  (4) Server -> forwards location to Client (PC/iOS/Android)
  (5) Server -> stores location record into DB
```

- **基站（Beacon）**：部署在室内环境中，恒定广播包含自身 ID 与信号信息；坐标与参数（高度补偿、参考功率等）保存在数据库中。
- **被定位终端**：任何支持蓝牙 4.0 的移动设备，扫描到多个基站的 RSSI 与基站 ID 后，将采集到的数据上传到服务器。
- **服务器**：接收终端上报数据、从 DB 获取基站坐标与补偿参数、调用定位算法得到坐标后推送到客户端并持久化。
- **数据库**：保存基站信息、参数、设备信息、定位历史与系统配置。
- **客户端**：负责可视化展示、告警与二次业务扩展。

流程注解：

- ① 基站周期性广播包含 `beacon_id` 与信号强度信息（RSSI 可由终端测量）。
- ② 终端将 `device_id`、若干 `{beacon_id, rssi}` 及时间戳上传到服务器。
- ③ 服务器查询 DB 中的基站坐标、参考功率 `p0`、环境衰减因子 `n` 及高度补偿 `h`，将 RSSI 转换为距离并进行定位计算。
- ④ 定位结果通过推送或 RMI/gRPC 等 RPC 机制发给客户端。
- ⑤ 定位结果写入 DB（包含时间戳、device_id、坐标、置信度等）。

**名词解释**

- **RSSI**: Received Signal Strength Indicator（接收信号强度），以 dBm 表示。
- **iBeacon**: Apple 提出的蓝牙低功耗广播协议，Beacon 通过广播 UUID/major/minor 与发射功率信息被识别。
- **p0**: 参考距离（通常取 1 米）处的接收功率（dBm）。
- **n（路径损耗指数）**: 环境衰减因子，表示信号随距离衰减速率，需现场校准。
- **高度补偿（h）**: 基站与终端在垂直方向上的高度差，定位时用于平面距离修正。
- **三边定位（Trilateration）**: 基于已知三个基站与对应距离的圆的交点来估算未知点。
- **加权三边定位**: 对每个基站的结果按精度（通常与距离反比）赋权后融合。
- **加权三角形质心**: 使用三基站构成的三角形交点（或近似交点）计算质心，再按权重融合。

**算法实现细节**

1. RSSI 到距离的基本模型（对数距离路径损耗模型）：

行文中常用公式为（取参考距离 $d_0 = 1m$）：

$$P(d) = P(d_0) - 10\,n\log_{10}(d/d_0)$$

由此解出距离 $d$：

$$d = d_0 \times 10^{\frac{P(d_0)-P(d)}{10n}}$$

其中 $P(d)$ 为接收功率（RSSI，单位 dBm），$P(d_0)$ 即 $p0$，$n$ 为路径损耗指数。

实测与校准：

- 在固定基站后，以 0.2m 为间隔在 14m 范围内采集 70 个点，每点采集 ~100 次 RSSI，去掉极端值后求平均以拟合 $p0$ 与 $n$（线性回归）。

2. 数据预处理：对同一基站采集到的多次 RSSI 按数值排序，去掉头尾若干极端值（基于百分比或固定数目），再取均值进行距离计算，以抵抗瞬时抖动与干扰。

3. 三边定位（最小二乘法线性化求解）简述：

给定 $n(\ge3)$ 个基站坐标 $(x_i,y_i)$ 与对应距离 $r_i$，未知点 $(x,y)$ 满足：

$$ (x-x_i)^2 + (y-y_i)^2 = r_i^2 \quad (i=1\ldots n) $$

用第 $n$ 个方程去减前 $n-1$ 个方程，得到线性方程组 $AX = b$，
其中 $X = [x, y]^T$，可用最小二乘解：

$$ X = (A^TA)^{-1}A^Tb $$

实际实现要注意数值稳定性（使用 QR 分解或奇异值分解 SVD 更稳健），以及当基站共线或几何条件差时的退化问题。

4. 加权三边定位：

对每一种从三基站组合得到的位置解 $X_j$，根据参与基站的距离大小给定权重 $w_j$（例如 $w\propto 1 / r_{avg}$ 或 $w\propto 1 / r_{avg}^2$），最终位置为加权平均：

$$ X = \frac{\sum_j w_j X_j}{\sum_j w_j} $$

5. 加权三角形质心：

对每组三个基站求出圆交点构成的三角形的质心（若交点不存在则跳过或采用最小化误差的近似方法），再按权重融合来自各组合的质心。

注意事项：当三个圆两两没有交点时需回退到最小二乘或仅使用部分基站。

**数据模型与消息格式（示例）**

- 上报消息（终端 -> 服务器，JSON 示例）：

```json
{
  "device_id": "dev-123",
  "timestamp": 1699999999,
  "readings": [
    { "beacon_id": "b-001", "rssi": -58 },
    { "beacon_id": "b-002", "rssi": -66 },
    { "beacon_id": "b-003", "rssi": -73 }
  ]
}
```

- 服务器->客户端 的定位推送示例：

```json
{
  "device_id": "dev-123",
  "timestamp": 1699999999,
  "location": { "x": 12.34, "y": 5.67, "z": 1.2 },
  "confidence": 0.87
}
```

**数据库表结构建议（简要）**

- `beacons`：`id, uuid, x, y, z, p0, n, created_at`
- `devices`：`device_id, meta...`
- `readings`：`id, device_id, beacon_id, rssi, ts`
- `locations`：`id, device_id, x, y, z, confidence, ts`
- `params`：全局或场景参数（如滤波门限、trim 百分比等）

**Java -> Rust 迁移对照与建议**

目标：在 Rust 中复现原有业务与算法，同时利用 Rust 的性能与安全性。以下为主要技术点的对照建议：

- 框架：
  - **Java Spring** -> **`axum` / `actix-web`**（推荐 `axum` 与 `tokio` 生态，轻量且现代）
- 高并发网络 I/O：
  - **Netty** -> **`tokio` + `hyper` / `tokio` + `axum`（http）或 `tokio` 原生 TCP/UDP**
- RPC（原 RMI）:
  - **RMI** -> **gRPC（`tonic`）/ JSON-RPC / WebSocket**（若已存在客户端，优先选择兼容的协议）
- 数据库：
  - **JDBC + Druid** -> **`sqlx`（异步 SQL + 连接池）或 `sea-orm`（ORM）**。`sqlx` 支持编译期检查 SQL，推荐生产使用。连接池由 `sqlx::Pool` 管理。
- 配置与依赖管理：
  - **Maven** -> **Cargo**（`Cargo.toml`）
- 序列化：
  - **Jackson / Gson** -> **`serde` + `serde_json`**
- 日志：
  - **SLF4J / Logback** -> **`tracing` / `tracing-subscriber`**
- 并发与任务调度：
  - **Java 线程池** -> **`tokio::spawn` + `tokio::task` + 限流信号量 (`tokio::sync::Semaphore`)**
- 配置：
  - **application.properties** -> **`config` crate`/`dotenv`+`toml`/`yaml`**

工程建议：

- 将定位算法模块化为独立的 Rust crate 或模块（例如 `algo::trilateration`、`algo::weighted`），方便单元测试与演进。
- 将网络层与算法层解耦：网络负责消息解析与队列，算法在 Worker 池中异步执行，结果通过 channel 回写到网络层与 DB 层。
- 使用 `serde` 定义严格的消息结构，便于前后端协同与回归测试。

**运行与配置**

- 推荐的环境变量：

  - `DATABASE_URL`：数据库连接字符串（例如 PostgreSQL/ MySQL）
  - `RUST_LOG`：日志等级（`info`, `debug`）
  - `CONFIG_PATH`：可选的 TOML/YAML 配置文件路径

- 常用命令（开发）：

```
cargo build
RUST_LOG=info DATABASE_URL="mysql://user:pass@127.0.0.1/db" cargo run
cargo test
```

**测试与验证**

- 单元测试：为每个算法模块编写 `cargo test` 覆盖边界情况（基站共线、缺失基站、测量噪声极大等）。
- 集成测试：模拟上报消息，校验定位输出与 DB 的写入。
- 线下校准：在受控环境中采集 RSSI 数据并用回归分析验证 `p0` 与 `n` 的拟合结果。

**性能与部署建议**

- 使用 `tokio` 多线程运行时（默认多线程特性）以支持高并发。
- 数据库连接池大小需与并发写入量匹配，避免过高连接数导致 DB 压力。
- 考虑使用消息队列（如 Kafka / RabbitMQ）解耦上报与定位计算，便于削峰与异步重试。
- 监控与指标：接入 `prometheus`（`prometheus` crate）用于指标导出（请求速率、平均延迟、算法成功率等）。
- 日志与错误追踪：使用 `tracing` 输出结构化日志并采集到集中日志系统（ELK/EFK）。

**迁移实施建议与步骤（高层）**

1. 将消息格式与 API 设计定稿（与客户端对齐，优先兼容原格式）。
2. 搭建基本网络服务骨架（`axum`/`hyper`），实现上报接收与简单回写。
3. 实现 RSSI -> 距离 的模块与单元测试，使用实测数据校准参数。
4. 实现三种定位算法模块，并用合成数据与实测数据验证准确性。
5. 集成 DB（`sqlx`），完成基站/设备/位置表的读写。
6. 性能优化（并发、批量写入、队列化）。
7. 制定回滚与灰度策略，逐步替换 Java 服务。

**后续工作建议**

- 增加在线参数学习模块：实时调整 `n` 与 `p0`，提高适应性。
- 增强异常处理与自愈（当基站故障或数据异常时自动降级算位或告警）。
- 提供可视化工具用于场景标定（帮助工程师完成 p0/n 校准）。

**附录：数学符号与常用公式速查**

- 对数距离模型： $P(d) = P(d_0) - 10 n \log_{10}(d/d_0)$
- 解距离： $d = d_0 \times 10^{(P(d_0)-P(d))/(10n)}$

---

文件位置：`doc/README.md`

如需我把 README 的内容作为代码注释、生成带有示例配置文件（`config.toml`）或创建演示脚本（`examples/`），我可以继续补充。
