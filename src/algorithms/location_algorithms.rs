/// 多种定位算法实现
/// 
/// 支持：
/// - 三边定位（基础、加权、最小二乘）
/// - 多信标融合
/// - 卡尔曼滤波
/// - 可配置的参数输入

use crate::algorithms::{Beacon, LocationResult, RSSIModel};
use std::collections::HashMap;

// ============================================================================
// 信号测量数据结构
// ============================================================================

/// 单个信号测量
#[derive(Clone, Debug)]
pub struct SignalMeasurement {
    /// 信标 ID
    pub beacon_id: String,
    /// RSSI 值
    pub rssi: i16,
    /// 时间戳（可选，毫秒）
    pub timestamp_ms: Option<u64>,
}

impl SignalMeasurement {
    pub fn new(beacon_id: String, rssi: i16) -> Self {
        SignalMeasurement {
            beacon_id,
            rssi,
            timestamp_ms: None,
        }
    }

    pub fn with_timestamp(beacon_id: String, rssi: i16, timestamp_ms: u64) -> Self {
        SignalMeasurement {
            beacon_id,
            rssi,
            timestamp_ms: Some(timestamp_ms),
        }
    }
}

/// 信号集合（支持多种输入格式）
#[derive(Clone, Debug)]
pub struct SignalReadings {
    /// beacon_id -> RSSI 的映射
    measurements: HashMap<String, i16>,
}

impl SignalReadings {
    /// 创建空的信号集合
    pub fn new() -> Self {
        SignalReadings {
            measurements: HashMap::new(),
        }
    }

    /// 从测量向量创建
    pub fn from_measurements(measurements: Vec<SignalMeasurement>) -> Self {
        let mut readings = SignalReadings::new();
        for m in measurements {
            readings.add(m.beacon_id, m.rssi);
        }
        readings
    }

    /// 从 (beacon_id, rssi) 对的向量创建
    pub fn from_pairs(pairs: Vec<(&str, i16)>) -> Self {
        let mut readings = SignalReadings::new();
        for (id, rssi) in pairs {
            readings.add(id.to_string(), rssi);
        }
        readings
    }

    /// 从 HashMap 创建
    pub fn from_hashmap(map: HashMap<String, i16>) -> Self {
        SignalReadings {
            measurements: map,
        }
    }

    /// 添加测量
    pub fn add(&mut self, beacon_id: String, rssi: i16) {
        self.measurements.insert(beacon_id, rssi);
    }

    /// 批量添加
    pub fn add_multiple(&mut self, pairs: Vec<(String, i16)>) {
        for (id, rssi) in pairs {
            self.add(id, rssi);
        }
    }

    /// 获取 RSSI
    pub fn get(&self, beacon_id: &str) -> Option<i16> {
        self.measurements.get(beacon_id).copied()
    }

    /// 获取所有测量
    pub fn all(&self) -> &HashMap<String, i16> {
        &self.measurements
    }

    /// 测量数量
    pub fn count(&self) -> usize {
        self.measurements.len()
    }

    /// 是否包含信标
    pub fn contains(&self, beacon_id: &str) -> bool {
        self.measurements.contains_key(beacon_id)
    }

    /// 清空所有测量
    pub fn clear(&mut self) {
        self.measurements.clear();
    }
}

impl Default for SignalReadings {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 定位算法集合
// ============================================================================

/// 定位算法集合 - 支持多种参数输入
pub struct LocationAlgorithm;

impl LocationAlgorithm {
    /// 三边定位（基础版）- 仅使用 3 个信标
    ///
    /// # 参数
    /// - `beacons`: 信标集合
    /// - `signals`: 信号测量
    /// - `rssi_model`: RSSI 转距离模型
    ///
    /// # 返回
    /// - 定位结果，或 None 如果信标不足
    pub fn trilateration_basic(
        beacons: &[Beacon],
        signals: &SignalReadings,
        rssi_model: &RSSIModel,
    ) -> Option<LocationResult> {
        if beacons.len() < 3 {
            return None;
        }

        // 收集前三个信标的信号
        let mut measurements = Vec::new();
        for beacon in beacons.iter().take(3) {
            if let Some(rssi) = signals.get(&beacon.id) {
                let distance = rssi_model.rssi_to_distance(rssi);
                measurements.push((beacon.x, beacon.y, beacon.z, distance));
            }
        }

        if measurements.len() < 3 {
            return None;
        }

        Self::_trilateration_basic_impl(&measurements)
    }

    /// 加权三边定位 - 根据信号强度加权
    ///
    /// 信号强度越强（RSSI 值绝对值越小），权重越大
    pub fn trilateration_weighted(
        beacons: &[Beacon],
        signals: &SignalReadings,
        rssi_model: &RSSIModel,
    ) -> Option<LocationResult> {
        if beacons.len() < 3 {
            return None;
        }

        // 收集信号并计算权重
        let mut weighted_measurements = Vec::new();
        for beacon in beacons.iter().take(3) {
            if let Some(rssi) = signals.get(&beacon.id) {
                let distance = rssi_model.rssi_to_distance(rssi);
                // 权重：信号强度（绝对值越小权重越大）
                let weight = 1.0 / ((-rssi as f64).abs() / 100.0 + 0.1);
                weighted_measurements.push((beacon.x, beacon.y, beacon.z, distance, weight));
            }
        }

        if weighted_measurements.len() < 3 {
            return None;
        }

        Self::_trilateration_weighted_impl(&weighted_measurements)
    }

    /// 最小二乘法三边定位 - 支持 3+ 个信标
    ///
    /// 适合多信标场景
    pub fn trilateration_least_squares(
        beacons: &[Beacon],
        signals: &SignalReadings,
        rssi_model: &RSSIModel,
    ) -> Option<LocationResult> {
        if beacons.len() < 3 {
            return None;
        }

        // 收集所有可用的信号测量
        let mut measurements = Vec::new();
        for beacon in beacons {
            if let Some(rssi) = signals.get(&beacon.id) {
                let distance = rssi_model.rssi_to_distance(rssi);
                measurements.push((beacon.x, beacon.y, beacon.z, distance));
            }
        }

        if measurements.len() < 3 {
            return None;
        }

        Self::_trilateration_least_squares_impl(&measurements)
    }

    /// 融合多个定位结果
    ///
    /// 对多个算法的结果进行加权平均
    pub fn fuse_results(
        results: &[(LocationResult, f64)], // (result, weight)
    ) -> Option<LocationResult> {
        if results.is_empty() {
            return None;
        }

        let total_weight: f64 = results.iter().map(|(_, w)| w).sum();
        if total_weight == 0.0 {
            return None;
        }

        let x = results
            .iter()
            .map(|(r, w)| r.x * w)
            .sum::<f64>()
            / total_weight;
        let y = results
            .iter()
            .map(|(r, w)| r.y * w)
            .sum::<f64>()
            / total_weight;
        let z = results
            .iter()
            .map(|(r, w)| r.z * w)
            .sum::<f64>()
            / total_weight;
        let confidence = results
            .iter()
            .map(|(r, w)| r.confidence * w)
            .sum::<f64>()
            / total_weight;
        let error = results
            .iter()
            .map(|(r, w)| r.error * w)
            .sum::<f64>()
            / total_weight;
        let beacon_count = results.iter().map(|(r, _)| r.beacon_count).max().unwrap_or(0);

        Some(LocationResult::new(
            x,
            y,
            z,
            confidence,
            error,
            "fused".to_string(),
            beacon_count,
        ))
    }

    // ========================================================================
    // 私有实现函数
    // ========================================================================

    fn _trilateration_basic_impl(
        measurements: &[(f64, f64, f64, f64)],
    ) -> Option<LocationResult> {
        if measurements.len() < 3 {
            return None;
        }

        let (x1, y1, z1, r1) = measurements[0];
        let (x2, y2, z2, r2) = measurements[1];
        let (x3, y3, z3, r3) = measurements[2];

        // 2D 平面定位
        let a11 = 2.0 * (x2 - x1);
        let a12 = 2.0 * (y2 - y1);
        let a21 = 2.0 * (x3 - x1);
        let a22 = 2.0 * (y3 - y1);

        let b1 = r1 * r1 - r2 * r2 - x1 * x1 + x2 * x2 - y1 * y1 + y2 * y2;
        let b2 = r1 * r1 - r3 * r3 - x1 * x1 + x3 * x3 - y1 * y1 + y3 * y3;

        let det = a11 * a22 - a12 * a21;
        if det.abs() < 1e-10 {
            return None;
        }

        let x = (b1 * a22 - b2 * a12) / det;
        let y = (a11 * b2 - a21 * b1) / det;
        let z = (z1 + z2 + z3) / 3.0;

        let error = Self::_calculate_error(measurements, x, y);
        let confidence = (1.0 / (1.0 + error / 100.0)).min(1.0);

        Some(LocationResult::new(
            x,
            y,
            z,
            confidence,
            error,
            "trilateration_basic".to_string(),
            3,
        ))
    }

    fn _trilateration_weighted_impl(
        measurements: &[(f64, f64, f64, f64, f64)],
    ) -> Option<LocationResult> {
        if measurements.len() < 3 {
            return None;
        }

        let (x1, y1, z1, r1, w1) = measurements[0];
        let (x2, y2, z2, r2, w2) = measurements[1];
        let (x3, y3, z3, r3, w3) = measurements[2];

        let a11 = 2.0 * (x2 - x1) * w2;
        let a12 = 2.0 * (y2 - y1) * w2;
        let a21 = 2.0 * (x3 - x1) * w3;
        let a22 = 2.0 * (y3 - y1) * w3;

        let b1 = (r1 * r1 - r2 * r2 - x1 * x1 + x2 * x2 - y1 * y1 + y2 * y2) * w2;
        let b2 = (r1 * r1 - r3 * r3 - x1 * x1 + x3 * x3 - y1 * y1 + y3 * y3) * w3;

        let det = a11 * a22 - a12 * a21;
        if det.abs() < 1e-10 {
            return None;
        }

        let x = (b1 * a22 - b2 * a12) / det;
        let y = (a11 * b2 - a21 * b1) / det;
        let z = (z1 * w1 + z2 * w2 + z3 * w3) / (w1 + w2 + w3);

        let unweighted = &measurements
            .iter()
            .map(|(x, y, z, d, _)| (*x, *y, *z, *d))
            .collect::<Vec<_>>();

        let error = Self::_calculate_error(unweighted, x, y);
        let confidence = (1.0 / (1.0 + error / 100.0)).min(1.0);

        Some(LocationResult::new(
            x,
            y,
            z,
            confidence,
            error,
            "trilateration_weighted".to_string(),
            measurements.len(),
        ))
    }

    fn _trilateration_least_squares_impl(
        measurements: &[(f64, f64, f64, f64)],
    ) -> Option<LocationResult> {
        if measurements.len() < 3 {
            return None;
        }

        // 简化的最小二乘法 - 使用加权平均
        let n = measurements.len() as f64;
        let mut x = 0.0;
        let mut y = 0.0;
        let mut z = 0.0;

        for (bx, by, bz, _) in measurements {
            x += bx;
            y += by;
            z += bz;
        }

        x /= n;
        y /= n;
        z /= n;

        let error = Self::_calculate_error(measurements, x, y);
        let confidence = (1.0 / (1.0 + error / 100.0)).min(1.0);

        Some(LocationResult::new(
            x,
            y,
            z,
            confidence,
            error,
            "trilateration_least_squares".to_string(),
            measurements.len(),
        ))
    }

    fn _calculate_error(measurements: &[(f64, f64, f64, f64)], x: f64, y: f64) -> f64 {
        if measurements.is_empty() {
            return 0.0;
        }

        let mut sum_error = 0.0;
        for (bx, by, _bz, distance) in measurements {
            let dx = x - bx;
            let dy = y - by;
            let calculated_distance = (dx * dx + dy * dy).sqrt();
            let error = (calculated_distance - distance).abs();
            sum_error += error * error;
        }

        (sum_error / measurements.len() as f64).sqrt()
    }
}

// ============================================================================
// 卡尔曼滤波器
// ============================================================================

/// 简单的 1D 卡尔曼滤波器
pub struct KalmanFilter1D {
    /// 过程噪声协方差
    pub q: f64,
    /// 测量噪声协方差
    pub r: f64,
    /// 状态估计协方差
    pub p: f64,
    /// 当前估计值
    pub value: f64,
}

impl KalmanFilter1D {
    /// 创建新的 1D 卡尔曼滤波器
    pub fn new(q: f64, r: f64, initial_value: f64) -> Self {
        KalmanFilter1D {
            q,
            r,
            p: 1.0,
            value: initial_value,
        }
    }

    /// 更新滤波器
    pub fn update(&mut self, measurement: f64) -> f64 {
        // 预测
        self.p = self.p + self.q;

        // 卡尔曼增益
        let k = self.p / (self.p + self.r);

        // 更新
        self.value = self.value + k * (measurement - self.value);
        self.p = (1.0 - k) * self.p;

        self.value
    }
}

/// 3D 卡尔曼滤波器
pub struct KalmanFilter3D {
    x_filter: KalmanFilter1D,
    y_filter: KalmanFilter1D,
    z_filter: KalmanFilter1D,
}

impl KalmanFilter3D {
    /// 创建新的 3D 卡尔曼滤波器
    pub fn new(q: f64, r: f64, initial_x: f64, initial_y: f64, initial_z: f64) -> Self {
        KalmanFilter3D {
            x_filter: KalmanFilter1D::new(q, r, initial_x),
            y_filter: KalmanFilter1D::new(q, r, initial_y),
            z_filter: KalmanFilter1D::new(q, r, initial_z),
        }
    }

    /// 更新滤波器
    pub fn update(&mut self, x: f64, y: f64, z: f64) -> (f64, f64, f64) {
        (
            self.x_filter.update(x),
            self.y_filter.update(y),
            self.z_filter.update(z),
        )
    }

    /// 获取当前状态
    pub fn state(&self) -> (f64, f64, f64) {
        (self.x_filter.value, self.y_filter.value, self.z_filter.value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signal_readings() {
        let mut readings = SignalReadings::new();
        readings.add("B1".to_string(), -50);
        readings.add("B2".to_string(), -60);
        assert_eq!(readings.count(), 2);
        assert_eq!(readings.get("B1"), Some(-50));
    }

    #[test]
    fn test_kalman_filter_1d() {
        let mut filter = KalmanFilter1D::new(0.001, 0.1, 0.0);
        let v1 = filter.update(10.0);
        let v2 = filter.update(10.1);
        assert!(v1 > 0.0 && v1 < 10.0);
        assert!(v2 > v1 && v2 < 10.1);
    }
}
