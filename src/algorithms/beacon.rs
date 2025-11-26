/// 蓝牙信标定义和相关数据结构

use std::collections::HashMap;

/// 单个蓝牙信标定义
#[derive(Clone, Debug)]
pub struct Beacon {
    /// 信标 MAC 地址或唯一标识符
    pub id: String,
    /// 信标友好名称
    pub name: String,
    /// X 坐标（单位可配置，默认厘米）
    pub x: f64,
    /// Y 坐标（单位可配置，默认厘米）
    pub y: f64,
    /// Z 坐标 - 高度（单位可配置，默认厘米）
    pub z: f64,
}

impl Beacon {
    /// 创建新的信标
    pub fn new(id: String, name: String, x: f64, y: f64, z: f64) -> Self {
        Beacon { id, name, x, y, z }
    }

    /// 从元组创建（简洁方式）
    pub fn from_tuple((id, name, x, y, z): (String, String, f64, f64, f64)) -> Self {
        Self::new(id, name, x, y, z)
    }

    /// 获取信标的 3D 坐标
    pub fn coordinates(&self) -> (f64, f64, f64) {
        (self.x, self.y, self.z)
    }

    /// 计算与另一信标的欧几里得距离
    pub fn distance_to(&self, other: &Beacon) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
}

/// 信标集合管理器 - 支持多个不同的信标配置集
pub struct BeaconSet {
    /// 信标 ID -> Beacon 的映射
    beacons: HashMap<String, Beacon>,
}

impl BeaconSet {
    /// 创建空的信标集合
    pub fn new() -> Self {
        BeaconSet {
            beacons: HashMap::new(),
        }
    }

    /// 从信标向量创建集合
    pub fn from_vec(beacons: Vec<Beacon>) -> Self {
        let mut set = BeaconSet::new();
        for beacon in beacons {
            set.add_beacon(beacon);
        }
        set
    }

    /// 添加信标
    pub fn add_beacon(&mut self, beacon: Beacon) {
        self.beacons.insert(beacon.id.clone(), beacon);
    }

    /// 添加多个信标
    pub fn add_beacons(&mut self, beacons: Vec<Beacon>) {
        for beacon in beacons {
            self.add_beacon(beacon);
        }
    }

    /// 获取信标
    pub fn get(&self, id: &str) -> Option<&Beacon> {
        self.beacons.get(id)
    }

    /// 获取可变引用的信标
    pub fn get_mut(&mut self, id: &str) -> Option<&mut Beacon> {
        self.beacons.get_mut(id)
    }

    /// 删除信标
    pub fn remove(&mut self, id: &str) -> Option<Beacon> {
        self.beacons.remove(id)
    }

    /// 获取所有信标
    pub fn all(&self) -> Vec<&Beacon> {
        self.beacons.values().collect()
    }

    /// 获取所有信标的克隆
    pub fn all_cloned(&self) -> Vec<Beacon> {
        self.beacons.values().cloned().collect()
    }

    /// 获取信标数量
    pub fn len(&self) -> usize {
        self.beacons.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.beacons.is_empty()
    }

    /// 清空所有信标
    pub fn clear(&mut self) {
        self.beacons.clear();
    }

    /// 迭代信标 ID 和信标
    pub fn iter(&self) -> impl Iterator<Item = (&String, &Beacon)> {
        self.beacons.iter()
    }
}

impl Default for BeaconSet {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_beacon_creation() {
        let beacon = Beacon::new("B1".to_string(), "Beacon1".to_string(), 0.0, 0.0, 100.0);
        assert_eq!(beacon.id, "B1");
        assert_eq!(beacon.x, 0.0);
    }

    #[test]
    fn test_beacon_distance() {
        let b1 = Beacon::new("B1".to_string(), "B1".to_string(), 0.0, 0.0, 0.0);
        let b2 = Beacon::new("B2".to_string(), "B2".to_string(), 3.0, 4.0, 0.0);
        assert_eq!(b1.distance_to(&b2), 5.0);
    }

    #[test]
    fn test_beacon_set() {
        let mut set = BeaconSet::new();
        let b1 = Beacon::new("B1".to_string(), "Beacon1".to_string(), 0.0, 0.0, 100.0);
        set.add_beacon(b1);
        assert_eq!(set.len(), 1);
        assert!(set.get("B1").is_some());
    }
}
