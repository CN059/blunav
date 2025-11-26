/// 蓝牙室内定位模块
/// 
/// 支持的功能：
/// - RSSI 转距离计算
/// - 多种定位算法（三边定位、加权三边、最小二乘等）
/// - 卡尔曼滤波时间序列融合
/// - 实时定位计算

/// 蓝牙信标定义
#[derive(Clone, Debug)]
pub struct Beacon {
    pub id: String,
    pub name: String,
    pub x: f64,      // 厘米
    pub y: f64,      // 厘米
    pub z: f64,      // 厘米（高度）
}

/// 定位结果
#[derive(Clone, Debug)]
pub struct LocationResult {
    pub x: f64,                    // 厘米
    pub y: f64,                    // 厘米
    pub z: f64,                    // 厘米
    pub confidence: f64,           // 0.0 ~ 1.0
    pub error: f64,                // 估计误差（厘米）
    pub method: String,            // 使用的算法
}

/// RSSI 转距离的参数
#[derive(Clone, Debug)]
pub struct RSSIModel {
    pub a: f64,      // 截距 (dBm)
    pub b: f64,      // 斜率
    pub n: f64,      // 路径损耗指数
}

impl RSSIModel {
    /// 从拟合参数创建模型
    /// 
    /// 模型公式：RSSI(d) = A + B * log10(d)
    pub fn new(a: f64, b: f64, n: f64) -> Self {
        RSSIModel { a, b, n }
    }

    /// 根据 RSSI 计算距离
    /// 
    /// 反解公式：d = 10^((RSSI - A) / B)
    pub fn rssi_to_distance(&self, rssi: i16) -> f64 {
        let rssi_f64 = rssi as f64;
        let exponent = (rssi_f64 - self.a) / self.b;
        10_f64.powf(exponent)
    }
}

/// ============================================================================
/// 定位算法实现
/// ============================================================================

/// 三边定位（基础版）- 仅使用三个最近的信标
pub fn trilateration_basic(
    beacons_with_distances: &[(f64, f64, f64, f64)], // [(x, y, z, distance), ...]
) -> Option<LocationResult> {
    if beacons_with_distances.len() < 3 {
        return None;
    }

    // 仅使用前三个信标
    let (x1, y1, z1, r1) = beacons_with_distances[0];
    let (x2, y2, z2, r2) = beacons_with_distances[1];
    let (x3, y3, z3, r3) = beacons_with_distances[2];

    // 2D 平面定位（忽略 z 轴，假设都在同一平面）
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

    // 估计 z 坐标（使用平均值）
    let z = (z1 + z2 + z3) / 3.0;

    let error = calculate_error(beacons_with_distances, x, y);
    let confidence = (1.0 / (1.0 + error / 100.0)).min(1.0);

    Some(LocationResult {
        x,
        y,
        z,
        confidence,
        error,
        method: "三边定位".to_string(),
    })
}

/// 加权三边定位 - 根据距离调整权重
pub fn trilateration_weighted(
    beacons_with_distances: &[(f64, f64, f64, f64)], // [(x, y, z, distance), ...]
) -> Option<LocationResult> {
    if beacons_with_distances.len() < 3 {
        return None;
    }

    // 计算权重（距离越近权重越大）
    let mut weighted_beacons = Vec::new();
    for &(x, y, z, d) in beacons_with_distances {
        let weight = if d < 50.0 {
            1.0
        } else {
            1.0 / (d * d / 10000.0)  // 按距离平方反比
        };
        weighted_beacons.push((x, y, z, d, weight));
    }

    // 使用前三个信标
    let (x1, y1, z1, r1, w1) = weighted_beacons[0];
    let (x2, y2, z2, r2, w2) = weighted_beacons[1];
    let (x3, y3, z3, r3, w3) = weighted_beacons[2];

    let a11 = 2.0 * (x2 - x1) * w1 * w2;
    let a12 = 2.0 * (y2 - y1) * w1 * w2;
    let a21 = 2.0 * (x3 - x1) * w1 * w3;
    let a22 = 2.0 * (y3 - y1) * w1 * w3;

    let b1 = (r1 * r1 - r2 * r2 - x1 * x1 + x2 * x2 - y1 * y1 + y2 * y2) * w1 * w2;
    let b2 = (r1 * r1 - r3 * r3 - x1 * x1 + x3 * x3 - y1 * y1 + y3 * y3) * w1 * w3;

    let det = a11 * a22 - a12 * a21;
    if det.abs() < 1e-10 {
        return None;
    }

    let x = (b1 * a22 - b2 * a12) / det;
    let y = (a11 * b2 - a21 * b1) / det;
    let z = (z1 + z2 + z3) / 3.0;

    let error = calculate_weighted_error(&weighted_beacons, x, y);
    let confidence = (1.0 / (1.0 + error / 100.0)).min(1.0);

    Some(LocationResult {
        x,
        y,
        z,
        confidence,
        error,
        method: "加权三边定位".to_string(),
    })
}

/// 最小二乘法定位 - 支持 4+ 信标
pub fn trilateration_least_squares(
    beacons_with_distances: &[(f64, f64, f64, f64)], // [(x, y, z, distance), ...]
) -> Option<LocationResult> {
    if beacons_with_distances.len() < 3 {
        return None;
    }

    // 初始估计
    let initial = trilateration_basic(beacons_with_distances)?;
    let mut x = initial.x;
    let mut y = initial.y;

    // 迭代改进（5 次迭代）
    for _ in 0..5 {
        let mut sum_wx = 0.0;
        let mut sum_wy = 0.0;
        let mut sum_wf = 0.0;
        let mut sum_w = 0.0;

        for &(bx, by, _, bd) in beacons_with_distances {
            let dist = ((x - bx).powi(2) + (y - by).powi(2)).sqrt();
            let error = dist - bd;
            let weight = 1.0 / (1.0 + (error.abs() / bd).max(0.1));

            let dx = if dist > 1e-6 {
                (x - bx) / dist
            } else {
                0.0
            };
            let dy = if dist > 1e-6 {
                (y - by) / dist
            } else {
                0.0
            };

            sum_wx += weight * dx;
            sum_wy += weight * dy;
            sum_wf += weight * error;
            sum_w += weight;
        }

        if sum_w < 1e-10 {
            break;
        }

        let step_size = 0.05;
        x -= step_size * sum_wx * sum_wf / sum_w;
        y -= step_size * sum_wy * sum_wf / sum_w;
    }

    let z = beacons_with_distances.iter().map(|(_, _, z, _)| z).sum::<f64>()
        / beacons_with_distances.len() as f64;

    let error = calculate_error(beacons_with_distances, x, y);
    let confidence = (1.0 / (1.0 + error / 100.0)).min(1.0);

    Some(LocationResult {
        x,
        y,
        z,
        confidence,
        error,
        method: format!("最小二乘法({}个信标)", beacons_with_distances.len()),
    })
}

/// 卡尔曼滤波器 - 用于平滑时间序列
pub struct KalmanFilter {
    pub x: f64,
    pub y: f64,
    pub vx: f64,  // x 速度
    pub vy: f64,  // y 速度
    p_xx: f64,
    p_yy: f64,
    p_vv: f64,
}

impl KalmanFilter {
    pub fn new(x: f64, y: f64) -> Self {
        KalmanFilter {
            x,
            y,
            vx: 0.0,
            vy: 0.0,
            p_xx: 100.0,
            p_yy: 100.0,
            p_vv: 1.0,
        }
    }

    pub fn update(&mut self, measured_x: f64, measured_y: f64, dt: f64) {
        // 预测
        self.x += self.vx * dt;
        self.y += self.vy * dt;
        self.p_xx += self.p_vv * dt * dt + 10.0;
        self.p_yy += self.p_vv * dt * dt + 10.0;

        // 更新
        let kx = self.p_xx / (self.p_xx + 50.0);
        let ky = self.p_yy / (self.p_yy + 50.0);

        let dx = measured_x - self.x;
        let dy = measured_y - self.y;

        self.x += kx * dx;
        self.y += ky * dy;

        self.vx = dx / (dt + 1e-10);
        self.vy = dy / (dt + 1e-10);

        self.p_xx = (1.0 - kx) * self.p_xx;
        self.p_yy = (1.0 - ky) * self.p_yy;
    }

    pub fn position(&self) -> (f64, f64) {
        (self.x, self.y)
    }
}

/// ============================================================================
/// 辅助函数
/// ============================================================================

fn calculate_error(beacons: &[(f64, f64, f64, f64)], x: f64, y: f64) -> f64 {
    let mut total_error = 0.0;
    for &(bx, by, _, bd) in beacons {
        let calc_dist = ((x - bx).powi(2) + (y - by).powi(2)).sqrt();
        total_error += (calc_dist - bd).abs();
    }
    total_error / beacons.len() as f64
}

fn calculate_weighted_error(beacons: &[(f64, f64, f64, f64, f64)], x: f64, y: f64) -> f64 {
    let mut total_error = 0.0;
    let mut total_weight = 0.0;
    for &(bx, by, _, bd, weight) in beacons {
        let calc_dist = ((x - bx).powi(2) + (y - by).powi(2)).sqrt();
        total_error += weight * (calc_dist - bd).abs();
        total_weight += weight;
    }
    if total_weight > 0.0 {
        total_error / total_weight
    } else {
        f64::INFINITY
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rssi_model() {
        // 根据 Python 的拟合结果
        // A = -49.656, B = -43.284, n = 4.328
        let model = RSSIModel::new(-49.656, -43.284, 4.328);

        // 在 1 米处，RSSI 应该是 A
        let d_at_ref = model.rssi_to_distance(-49);
        println!("RSSI -49 dBm 对应距离: {:.2} cm", d_at_ref);
    }
}
