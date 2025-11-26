blunav/
├── Cargo.toml
├── src/
│ ├── main.rs # 应用入口
│ ├── config.rs # 配置管理（config crate）
│ ├── db/ # 数据库层
│ │ ├── mod.rs
│ │ ├── models.rs # 数据模型（serde + sqlx）
│ │ └── queries.rs # SQL 查询（sqlx）
│ ├── network/ # 网络层
│ │ ├── mod.rs
│ │ ├── handler.rs # HTTP 路由处理（axum）
│ │ └── message.rs # 消息结构（serde）
│ ├── algo/ # 定位算法核心
│ │ ├── mod.rs
│ │ ├── distance.rs # RSSI -> 距离转换
│ │ ├── trilateration.rs # 三边定位（nalgebra）
│ │ ├── weighted.rs # 加权三边
│ │ └── centroid.rs # 加权质心
│ ├── service/ # 业务逻辑层
│ │ ├── mod.rs
│ │ ├── location.rs # 定位服务编排
│ │ └── calibration.rs # 参数校准（可选）
│ ├── metrics.rs # 监控指标（prometheus）
│ ├── error.rs # 自定义错误类型（thiserror）
│ └── lib.rs # 库公共接口
├── tests/ # 集成测试
├── examples/ # 示例代码
└── benches/ # 性能基准（criterion）
