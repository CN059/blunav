/// 定位结果数据结构
/// 
/// 包含定位输出的各种信息和元数据

use std::fmt;
use chrono::{DateTime, Utc};

/// 定位结果
#[derive(Clone, Debug)]
pub struct LocationResult {
    /// X 坐标
    pub x: f64,
    /// Y 坐标
    pub y: f64,
    /// Z 坐标（高度）
    pub z: f64,
    /// 定位置信度 (0.0 ~ 1.0)
    pub confidence: f64,
    /// 估计误差（单位与模型一致）
    pub error: f64,
    /// 使用的算法名称
    pub method: String,
    /// 参与定位的信标数量
    pub beacon_count: usize,
    /// 时间戳
    pub timestamp: DateTime<Utc>,
}

impl LocationResult {
    /// 创建新的定位结果
    pub fn new(
        x: f64,
        y: f64,
        z: f64,
        confidence: f64,
        error: f64,
        method: String,
        beacon_count: usize,
    ) -> Self {
        LocationResult {
            x,
            y,
            z,
            confidence: confidence.max(0.0).min(1.0),
            error,
            method,
            beacon_count,
            timestamp: Utc::now(),
        }
    }

    /// 创建具有自定义时间戳的结果
    pub fn with_timestamp(
        x: f64,
        y: f64,
        z: f64,
        confidence: f64,
        error: f64,
        method: String,
        beacon_count: usize,
        timestamp: DateTime<Utc>,
    ) -> Self {
        LocationResult {
            x,
            y,
            z,
            confidence: confidence.max(0.0).min(1.0),
            error,
            method,
            beacon_count,
            timestamp,
        }
    }

    /// 获取 2D 坐标
    pub fn xy(&self) -> (f64, f64) {
        (self.x, self.y)
    }

    /// 获取 3D 坐标
    pub fn xyz(&self) -> (f64, f64, f64) {
        (self.x, self.y, self.z)
    }

    /// 与另一结果的欧几里得距离
    pub fn distance_to(&self, other: &LocationResult) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    /// 与另一结果的 2D 距离
    pub fn distance_2d_to(&self, other: &LocationResult) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }

    /// 质量评分（基于置信度和误差）
    pub fn quality_score(&self) -> f64 {
        let confidence_factor = self.confidence;
        let error_factor = 1.0 / (1.0 + self.error / 100.0);
        (confidence_factor + error_factor) / 2.0
    }

    /// 是否是高质量结果（置信度 > 0.7 且误差 < 100）
    pub fn is_high_quality(&self) -> bool {
        self.confidence > 0.7 && self.error < 100.0
    }

    /// 获取详细描述
    pub fn detailed_description(&self) -> String {
        format!(
            "位置: ({:.2}, {:.2}, {:.2}), 置信度: {:.1}%, 误差: {:.2}, 方法: {}, 信标数: {}",
            self.x,
            self.y,
            self.z,
            self.confidence * 100.0,
            self.error,
            self.method,
            self.beacon_count
        )
    }
}

impl fmt::Display for LocationResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "({:.2}, {:.2}, {:.2}) [{:.1}%]",
            self.x,
            self.y,
            self.z,
            self.confidence * 100.0
        )
    }
}

/// 定位结果序列（用于时间序列处理）
#[derive(Clone, Debug)]
pub struct LocationSequence {
    /// 结果列表
    results: Vec<LocationResult>,
}

impl LocationSequence {
    /// 创建空序列
    pub fn new() -> Self {
        LocationSequence {
            results: Vec::new(),
        }
    }

    /// 添加结果
    pub fn push(&mut self, result: LocationResult) {
        self.results.push(result);
    }

    /// 获取最后一个结果
    pub fn last(&self) -> Option<&LocationResult> {
        self.results.last()
    }

    /// 获取所有结果
    pub fn all(&self) -> &[LocationResult] {
        &self.results
    }

    /// 结果数量
    pub fn len(&self) -> usize {
        self.results.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.results.is_empty()
    }

    /// 获取平均位置
    pub fn average_position(&self) -> Option<LocationResult> {
        if self.results.is_empty() {
            return None;
        }

        let count = self.results.len() as f64;
        let x = self.results.iter().map(|r| r.x).sum::<f64>() / count;
        let y = self.results.iter().map(|r| r.y).sum::<f64>() / count;
        let z = self.results.iter().map(|r| r.z).sum::<f64>() / count;
        let avg_confidence = self.results.iter().map(|r| r.confidence).sum::<f64>() / count;
        let avg_error = self.results.iter().map(|r| r.error).sum::<f64>() / count;

        Some(LocationResult::new(
            x,
            y,
            z,
            avg_confidence,
            avg_error,
            "average".to_string(),
            0,
        ))
    }

    /// 获取最近 N 个结果的平均位置
    pub fn average_last_n(&self, n: usize) -> Option<LocationResult> {
        if self.results.is_empty() {
            return None;
        }

        let start = if self.results.len() > n {
            self.results.len() - n
        } else {
            0
        };

        let slice = &self.results[start..];
        let count = slice.len() as f64;

        let x = slice.iter().map(|r| r.x).sum::<f64>() / count;
        let y = slice.iter().map(|r| r.y).sum::<f64>() / count;
        let z = slice.iter().map(|r| r.z).sum::<f64>() / count;
        let avg_confidence = slice.iter().map(|r| r.confidence).sum::<f64>() / count;
        let avg_error = slice.iter().map(|r| r.error).sum::<f64>() / count;

        Some(LocationResult::new(
            x,
            y,
            z,
            avg_confidence,
            avg_error,
            format!("average_last_{}", n),
            0,
        ))
    }

    /// 清空序列
    pub fn clear(&mut self) {
        self.results.clear();
    }
}

impl Default for LocationSequence {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_location_result_creation() {
        let result = LocationResult::new(100.0, 200.0, 50.0, 0.85, 10.0, "method".to_string(), 3);
        assert_eq!(result.x, 100.0);
        assert_eq!(result.confidence, 0.85);
    }

    #[test]
    fn test_distance_calculation() {
        let r1 = LocationResult::new(0.0, 0.0, 0.0, 0.8, 10.0, "m".to_string(), 3);
        let r2 = LocationResult::new(3.0, 4.0, 0.0, 0.8, 10.0, "m".to_string(), 3);
        assert_eq!(r1.distance_to(&r2), 5.0);
    }

    #[test]
    fn test_location_sequence() {
        let mut seq = LocationSequence::new();
        seq.push(LocationResult::new(100.0, 200.0, 50.0, 0.8, 10.0, "m".to_string(), 3));
        seq.push(LocationResult::new(110.0, 210.0, 50.0, 0.8, 10.0, "m".to_string(), 3));

        assert_eq!(seq.len(), 2);
        let avg = seq.average_position().unwrap();
        assert!((avg.x - 105.0).abs() < 0.1);
    }
}
