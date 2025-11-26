/// 定位算法模块
/// 
/// 该模块提供多种室内定位算法的实现，支持：
/// - 多种参数输入格式（灵活适配不同数据源）
/// - 多种定位算法（三边定位、加权定位、最小二乘等）
/// - 实时位置融合和平滑处理
/// - 可配置的模型参数

pub mod location_algorithms;
pub mod rssi_model;
pub mod beacon;
pub mod results;

pub use location_algorithms::*;
pub use rssi_model::*;
pub use beacon::*;
pub use results::*;
