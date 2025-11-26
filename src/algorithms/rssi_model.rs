/// RSSI 到距离转换模型
/// 
/// 支持多种 RSSI 模型参数化方式，灵活适配不同数据源

use std::fmt;

/// 定位计量单位
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DistanceUnit {
    /// 厘米
    Centimeter,
    /// 米
    Meter,
    /// 毫米
    Millimeter,
}

/// RSSI 转距离模型 - 支持多种参数化方式
#[derive(Clone, Debug)]
pub struct RSSIModel {
    /// 截距 A (dBm) - 1 米处的参考功率
    pub a: f64,
    /// 斜率 B - 衰减速率
    pub b: f64,
    /// 路径损耗指数 n（某些模型使用）
    pub n: f64,
    /// 距离单位
    pub unit: DistanceUnit,
    /// 模型名称/类型
    pub model_type: String,
}

impl RSSIModel {
    /// 创建对数路径损耗模型
    /// 
    /// 公式: RSSI(d) = A + B * log10(d)
    /// 
    /// # 参数
    /// - `a`: 截距 (dBm)
    /// - `b`: 斜率
    /// - `unit`: 距离单位
    pub fn log_distance(a: f64, b: f64, unit: DistanceUnit) -> Self {
        RSSIModel {
            a,
            b,
            n: 0.0,
            unit,
            model_type: "log_distance".to_string(),
        }
    }

    /// 创建自由空间路径损耗模型
    /// 
    /// 公式: RSSI(d) = A - 20*log10(d) - 20*log10(f)
    /// 
    /// # 参数
    /// - `a`: 参考功率 (dBm)
    /// - `unit`: 距离单位
    pub fn free_space(a: f64, unit: DistanceUnit) -> Self {
        RSSIModel {
            a,
            b: -20.0,
            n: 2.0,
            unit,
            model_type: "free_space".to_string(),
        }
    }

    /// 创建通用对数正态阴影模型（Log Normal Shadow）
    /// 
    /// # 参数
    /// - `a`: 参考功率 (dBm at 1m)
    /// - `n`: 路径损耗指数
    /// - `unit`: 距离单位
    pub fn log_normal_shadow(a: f64, n: f64, unit: DistanceUnit) -> Self {
        RSSIModel {
            a,
            b: -10.0 * n,
            n,
            unit,
            model_type: "log_normal_shadow".to_string(),
        }
    }

    /// 创建自定义模型
    pub fn custom(a: f64, b: f64, n: f64, model_type: impl Into<String>, unit: DistanceUnit) -> Self {
        RSSIModel {
            a,
            b,
            n,
            unit,
            model_type: model_type.into(),
        }
    }

    /// 从 Python 拟合参数创建模型
    /// Python 输出格式: A=-49.656, B=-43.284, n=4.328
    pub fn from_python_fit(a: f64, b: f64, n: f64, unit: DistanceUnit) -> Self {
        RSSIModel {
            a,
            b,
            n,
            unit,
            model_type: "python_fit".to_string(),
        }
    }

    /// 根据 RSSI 计算距离
    /// 
    /// 反解对数距离模型: d = 10^((RSSI - A) / B)
    pub fn rssi_to_distance(&self, rssi: i16) -> f64 {
        let rssi_f64 = rssi as f64;
        let exponent = (rssi_f64 - self.a) / self.b;
        let distance = 10_f64.powf(exponent);
        self.convert_distance(distance, DistanceUnit::Meter)
    }

    /// 根据 RSSI 和任意 RSSI 值计算距离
    pub fn rssi_to_distance_f64(&self, rssi: f64) -> f64 {
        let exponent = (rssi - self.a) / self.b;
        let distance = 10_f64.powf(exponent);
        self.convert_distance(distance, DistanceUnit::Meter)
    }

    /// 根据距离计算 RSSI
    pub fn distance_to_rssi(&self, distance: f64) -> f64 {
        let distance_in_meters = self.convert_distance_from(distance);
        if distance_in_meters <= 0.0 {
            return f64::NEG_INFINITY;
        }
        self.a + self.b * distance_in_meters.log10()
    }

    /// 单位转换 - 从标准米转换为目标单位
    fn convert_distance(&self, distance: f64, from_unit: DistanceUnit) -> f64 {
        if from_unit == self.unit {
            return distance;
        }
        match (from_unit, self.unit) {
            (DistanceUnit::Meter, DistanceUnit::Centimeter) => distance * 100.0,
            (DistanceUnit::Meter, DistanceUnit::Millimeter) => distance * 1000.0,
            (DistanceUnit::Centimeter, DistanceUnit::Meter) => distance / 100.0,
            (DistanceUnit::Centimeter, DistanceUnit::Millimeter) => distance * 10.0,
            (DistanceUnit::Millimeter, DistanceUnit::Meter) => distance / 1000.0,
            (DistanceUnit::Millimeter, DistanceUnit::Centimeter) => distance / 10.0,
            _ => distance, // Same unit
        }
    }

    /// 将距离从目标单位转换为米
    fn convert_distance_from(&self, distance: f64) -> f64 {
        match self.unit {
            DistanceUnit::Meter => distance,
            DistanceUnit::Centimeter => distance / 100.0,
            DistanceUnit::Millimeter => distance / 1000.0,
        }
    }

    /// 转换单位
    pub fn convert_to_unit(&self, distance: f64, target_unit: DistanceUnit) -> f64 {
        // 先将距离从当前单位转换为米
        let meters = self.convert_distance_from(distance);
        // 再从米转换为目标单位
        match target_unit {
            DistanceUnit::Meter => meters.max(0.0),
            DistanceUnit::Centimeter => (meters * 100.0).max(0.0),
            DistanceUnit::Millimeter => (meters * 1000.0).max(0.0),
        }
    }

    /// 验证 RSSI 模型的合理性
    pub fn validate(&self) -> Result<(), String> {
        if self.b >= 0.0 {
            return Err("斜率 B 应为负数（RSSI 随距离增加而减小）".to_string());
        }
        if self.a > 0.0 {
            return Err("截距 A 通常为负（功率以 dBm 表示）".to_string());
        }
        Ok(())
    }

    /// 获取模型描述
    pub fn description(&self) -> String {
        format!(
            "RSSI模型 [{}] - A={:.2} dBm, B={:.2}, n={:.2}, 单位: {:?}",
            self.model_type, self.a, self.b, self.n, self.unit
        )
    }
}

impl Default for RSSIModel {
    fn default() -> Self {
        // 默认使用通常的 BLE 参数
        RSSIModel::log_distance(-49.0, -40.0, DistanceUnit::Centimeter)
    }
}

impl fmt::Display for RSSIModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_distance_model() {
        let model = RSSIModel::log_distance(-49.656, -43.284, DistanceUnit::Centimeter);
        
        // 在 1 米处，RSSI 应该等于 A
        let dist_1m = model.rssi_to_distance(-49);
        assert!((dist_1m - 100.0).abs() < 50.0); // 大约 100 厘米
    }

    #[test]
    fn test_distance_to_rssi() {
        let model = RSSIModel::log_distance(-50.0, -40.0, DistanceUnit::Centimeter);
        let rssi = model.distance_to_rssi(100.0); // 100 cm = 1 m
        assert!((rssi - (-50.0)).abs() < 1.0);
    }

    #[test]
    fn test_unit_conversion() {
        let model = RSSIModel::log_distance(-50.0, -40.0, DistanceUnit::Centimeter);
        let distance_cm = 100.0;  // 100 cm
        let distance_m = model.convert_to_unit(distance_cm, DistanceUnit::Meter);
        // 100 cm = 1 m，所以应该转换为 1.0 m
        assert!((distance_m - 1.0).abs() < 0.01);
    }
}
