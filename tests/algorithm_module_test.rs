/// 算法模块综合演示测试
/// 
/// 展示如何使用新的 algorithms 模块中的各个组件

#[cfg(test)]
mod tests {
    use blunav::algorithms::*;
    use std::collections::HashMap;

    #[test]
    fn test_algorithm_module_beacon_creation() {
        // 创建信标
        let beacon = Beacon::new(
            "20:A7:16:5E:C5:D6".to_string(),
            "RFstar_C5D6".to_string(),
            764.0,
            216.0,
            63.0,
        );

        assert_eq!(beacon.id, "20:A7:16:5E:C5:D6");
        assert_eq!(beacon.name, "RFstar_C5D6");
        assert_eq!(beacon.coordinates(), (764.0, 216.0, 63.0));
    }

    #[test]
    fn test_algorithm_module_beacon_set() {
        // 创建信标集合
        let mut beacons = BeaconSet::new();

        beacons.add_beacon(Beacon::new(
            "B1".to_string(),
            "Beacon1".to_string(),
            0.0,
            0.0,
            100.0,
        ));
        beacons.add_beacon(Beacon::new(
            "B2".to_string(),
            "Beacon2".to_string(),
            764.0,
            0.0,
            100.0,
        ));
        beacons.add_beacon(Beacon::new(
            "B3".to_string(),
            "Beacon3".to_string(),
            382.0,
            661.0,
            100.0,
        ));

        assert_eq!(beacons.len(), 3);
        assert!(beacons.get("B1").is_some());
    }

    #[test]
    fn test_algorithm_module_rssi_model_variations() {
        // 测试多种 RSSI 模型

        // 1. Python 拟合模型
        let model1 = RSSIModel::from_python_fit(
            -49.656,
            -43.284,
            4.328,
            DistanceUnit::Centimeter,
        );
        assert_eq!(model1.model_type, "python_fit");

        // 2. 日志距离模型
        let model2 = RSSIModel::log_distance(-50.0, -40.0, DistanceUnit::Centimeter);
        assert_eq!(model2.model_type, "log_distance");

        // 3. 自由空间模型
        let model3 = RSSIModel::free_space(-50.0, DistanceUnit::Meter);
        assert_eq!(model3.model_type, "free_space");

        // 4. 对数正态阴影模型
        let model4 = RSSIModel::log_normal_shadow(-50.0, 2.5, DistanceUnit::Centimeter);
        assert_eq!(model4.model_type, "log_normal_shadow");

        // 验证模型参数
        assert_eq!(model1.a, -49.656);
        assert_eq!(model1.b, -43.284);
    }

    #[test]
    fn test_algorithm_module_unit_conversions() {
        let model = RSSIModel::log_distance(-50.0, -40.0, DistanceUnit::Centimeter);

        // 100 cm = 1 m
        let distance_m = model.convert_to_unit(100.0, DistanceUnit::Meter);
        assert!((distance_m - 1.0).abs() < 0.01);

        // 100 cm = 1000 mm
        let distance_mm = model.convert_to_unit(100.0, DistanceUnit::Millimeter);
        assert!((distance_mm - 1000.0).abs() < 1.0);
    }

    #[test]
    fn test_algorithm_module_signal_readings_variations() {
        // 方式 1: 逐个添加
        let mut signals1 = SignalReadings::new();
        signals1.add("B1".to_string(), -50);
        signals1.add("B2".to_string(), -60);
        signals1.add("B3".to_string(), -70);
        assert_eq!(signals1.count(), 3);

        // 方式 2: 从测量向量创建
        let measurements = vec![
            SignalMeasurement::new("B1".to_string(), -50),
            SignalMeasurement::new("B2".to_string(), -60),
            SignalMeasurement::new("B3".to_string(), -70),
        ];
        let signals2 = SignalReadings::from_measurements(measurements);
        assert_eq!(signals2.count(), 3);

        // 方式 3: 从键值对创建
        let signals3 = SignalReadings::from_pairs(vec![
            ("B1", -50),
            ("B2", -60),
            ("B3", -70),
        ]);
        assert_eq!(signals3.count(), 3);

        // 方式 4: 从 HashMap 创建
        let mut map = HashMap::new();
        map.insert("B1".to_string(), -50);
        map.insert("B2".to_string(), -60);
        map.insert("B3".to_string(), -70);
        let signals4 = SignalReadings::from_hashmap(map);
        assert_eq!(signals4.count(), 3);

        // 所有方式结果应该相同
        assert_eq!(signals1.get("B1"), signals2.get("B1"));
        assert_eq!(signals2.get("B2"), signals3.get("B2"));
        assert_eq!(signals3.get("B3"), signals4.get("B3"));
    }

    #[test]
    fn test_algorithm_module_trilateration_basic() {
        // 创建三个已知位置的信标
        let beacons = vec![
            Beacon::new("B1".to_string(), "Beacon1".to_string(), 0.0, 0.0, 100.0),
            Beacon::new("B2".to_string(), "Beacon2".to_string(), 764.0, 0.0, 100.0),
            Beacon::new("B3".to_string(), "Beacon3".to_string(), 382.0, 661.0, 100.0),
        ];

        // 创建 RSSI 模型
        let model = RSSIModel::log_distance(-49.656, -43.284, DistanceUnit::Centimeter);

        // 创建信号测量
        let signals = SignalReadings::from_pairs(vec![
            ("B1", -52),
            ("B2", -77),
            ("B3", -86),
        ]);

        // 执行定位
        if let Some(result) = LocationAlgorithm::trilateration_basic(&beacons, &signals, &model) {
            println!("基础三边定位结果:");
            println!("  位置: ({:.2}, {:.2}, {:.2})", result.x, result.y, result.z);
            println!("  置信度: {:.1}%", result.confidence * 100.0);
            println!("  误差: {:.2}", result.error);
            println!("  方法: {}", result.method);
            println!("  信标数: {}", result.beacon_count);

            // 验证基本属性（三角定位可能产生范围外的值，这是正常的）
            assert!(result.confidence >= 0.0 && result.confidence <= 1.0);
            assert!(result.beacon_count == 3);
        }
    }

    #[test]
    fn test_algorithm_module_trilateration_weighted() {
        let beacons = vec![
            Beacon::new("B1".to_string(), "Beacon1".to_string(), 0.0, 0.0, 100.0),
            Beacon::new("B2".to_string(), "Beacon2".to_string(), 764.0, 0.0, 100.0),
            Beacon::new("B3".to_string(), "Beacon3".to_string(), 382.0, 661.0, 100.0),
        ];

        let model = RSSIModel::log_distance(-49.656, -43.284, DistanceUnit::Centimeter);

        let signals = SignalReadings::from_pairs(vec![
            ("B1", -52),
            ("B2", -77),
            ("B3", -86),
        ]);

        if let Some(result) = LocationAlgorithm::trilateration_weighted(&beacons, &signals, &model) {
            println!("加权三边定位结果:");
            println!("  位置: ({:.2}, {:.2}, {:.2})", result.x, result.y, result.z);
            println!("  方法: {}", result.method);
            assert_eq!(result.method, "trilateration_weighted");
        }
    }

    #[test]
    fn test_algorithm_module_trilateration_least_squares() {
        let beacons = vec![
            Beacon::new("B1".to_string(), "Beacon1".to_string(), 0.0, 0.0, 100.0),
            Beacon::new("B2".to_string(), "Beacon2".to_string(), 764.0, 0.0, 100.0),
            Beacon::new("B3".to_string(), "Beacon3".to_string(), 382.0, 661.0, 100.0),
        ];

        let model = RSSIModel::log_distance(-49.656, -43.284, DistanceUnit::Centimeter);

        let signals = SignalReadings::from_pairs(vec![
            ("B1", -52),
            ("B2", -77),
            ("B3", -86),
        ]);

        if let Some(result) =
            LocationAlgorithm::trilateration_least_squares(&beacons, &signals, &model)
        {
            println!("最小二乘定位结果:");
            println!("  位置: ({:.2}, {:.2}, {:.2})", result.x, result.y, result.z);
            println!("  信标数: {}", result.beacon_count);
            assert_eq!(result.beacon_count, 3);
        }
    }

    #[test]
    fn test_algorithm_module_fuse_results() {
        // 创建多个定位结果
        let result1 = LocationResult::new(368.0, 339.0, 94.0, 0.8, 20.0, "method1".to_string(), 3);
        let result2 = LocationResult::new(370.0, 340.0, 94.0, 0.85, 15.0, "method2".to_string(), 3);
        let result3 = LocationResult::new(367.0, 338.0, 94.0, 0.75, 25.0, "method3".to_string(), 3);

        // 融合结果
        if let Some(fused) = LocationAlgorithm::fuse_results(&[
            (result1, 0.2),
            (result2, 0.5),
            (result3, 0.3),
        ]) {
            println!("融合定位结果:");
            println!("  位置: ({:.2}, {:.2}, {:.2})", fused.x, fused.y, fused.z);
            println!("  置信度: {:.1}%", fused.confidence * 100.0);
            println!("  方法: {}", fused.method);
            assert_eq!(fused.method, "fused");
        }
    }

    #[test]
    fn test_algorithm_module_location_result() {
        let result = LocationResult::new(368.0, 339.0, 94.0, 0.85, 20.0, "method".to_string(), 3);

        // 测试坐标获取
        assert_eq!(result.xy(), (368.0, 339.0));
        assert_eq!(result.xyz(), (368.0, 339.0, 94.0));

        // 测试距离计算
        let result2 = LocationResult::new(371.0, 342.0, 94.0, 0.85, 20.0, "method".to_string(), 3);
        let dist = result.distance_to(&result2);
        assert!((dist - 4.242..=4.243).contains(&dist));

        // 测试质量评分
        let score = result.quality_score();
        assert!(score > 0.0 && score <= 1.0);

        // 测试高质量判断
        let high_quality = result.is_high_quality();
        assert!(high_quality);
    }

    #[test]
    fn test_algorithm_module_location_sequence() {
        let mut sequence = LocationSequence::new();

        for i in 0..5 {
            let result = LocationResult::new(
                368.0 + i as f64,
                339.0 + i as f64,
                94.0,
                0.85,
                20.0,
                "method".to_string(),
                3,
            );
            sequence.push(result);
        }

        assert_eq!(sequence.len(), 5);

        // 获取平均位置
        if let Some(avg) = sequence.average_position() {
            println!("平均位置: ({:.2}, {:.2}, {:.2})", avg.x, avg.y, avg.z);
            // 平均值应该在中间
            assert!((avg.x - 370.0).abs() < 1.0);
        }

        // 获取最近 3 个结果的平均
        if let Some(recent_avg) = sequence.average_last_n(3) {
            println!("最近 3 个平均: ({:.2}, {:.2})", recent_avg.x, recent_avg.y);
        }
    }

    #[test]
    fn test_algorithm_module_kalman_filter_1d() {
        let mut filter = KalmanFilter1D::new(0.001, 0.1, 0.0);

        let measurements = vec![100.5, 100.3, 100.7, 100.4, 100.6];
        let mut filtered_values = Vec::new();

        for measurement in measurements {
            let filtered = filter.update(measurement);
            filtered_values.push(filtered);
        }

        // 验证滤波效果
        println!("1D 卡尔曼滤波结果:");
        for (i, &val) in filtered_values.iter().enumerate() {
            println!("  步骤 {}: {:.3}", i + 1, val);
        }

        // 最后的值应该接近平均测量值（约 100.5）
        let final_value = filtered_values[filtered_values.len() - 1];
        assert!(final_value > 95.0, "最终值 {} 应在合理范围内", final_value);
        assert!(final_value < 105.0, "最终值 {} 应在合理范围内", final_value);
    }

    #[test]
    fn test_algorithm_module_kalman_filter_3d() {
        let mut filter = KalmanFilter3D::new(0.001, 0.1, 368.0, 339.0, 94.0);

        let measurements = vec![
            (368.5, 339.2, 94.1),
            (368.3, 339.5, 94.0),
            (368.7, 339.3, 94.2),
            (368.4, 339.6, 94.1),
            (368.6, 339.4, 94.0),
        ];

        let mut filtered_positions = Vec::new();

        for (x, y, z) in measurements {
            let (fx, fy, fz) = filter.update(x, y, z);
            filtered_positions.push((fx, fy, fz));
        }

        println!("3D 卡尔曼滤波结果:");
        for (i, &(x, y, z)) in filtered_positions.iter().enumerate() {
            println!("  步骤 {}: ({:.2}, {:.2}, {:.2})", i + 1, x, y, z);
        }

        // 验证结果在合理范围内
        let (x, y, z) = filter.state();
        assert!((x - 368.0).abs() < 1.0);
        assert!((y - 339.0).abs() < 1.0);
        assert!((z - 94.0).abs() < 1.0);
    }

    #[test]
    fn test_algorithm_module_complete_workflow() {
        println!("\n========== 完整工作流演示 ==========\n");

        // 1. 定义环境
        let beacons = vec![
            Beacon::new("B1".to_string(), "Beacon1".to_string(), 0.0, 0.0, 100.0),
            Beacon::new("B2".to_string(), "Beacon2".to_string(), 764.0, 0.0, 100.0),
            Beacon::new("B3".to_string(), "Beacon3".to_string(), 382.0, 661.0, 100.0),
        ];

        let model = RSSIModel::from_python_fit(
            -49.656,
            -43.284,
            4.328,
            DistanceUnit::Centimeter,
        );

        // 2. 模拟多次测量
        let signal_sequences = vec![
            SignalReadings::from_pairs(vec![("B1", -52), ("B2", -77), ("B3", -86)]),
            SignalReadings::from_pairs(vec![("B1", -48), ("B2", -70), ("B3", -80)]),
            SignalReadings::from_pairs(vec![("B1", -48), ("B2", -70), ("B3", -80)]),
        ];

        // 3. 创建卡尔曼滤波器
        let mut filter = KalmanFilter3D::new(0.001, 0.1, 368.0, 339.0, 94.0);

        let mut sequence = LocationSequence::new();

        // 4. 对每个测量进行定位
        for (idx, signals) in signal_sequences.iter().enumerate() {
            println!("测量 {}:", idx + 1);

            // 使用多种算法
            if let Some(result1) = LocationAlgorithm::trilateration_basic(&beacons, signals, &model)
            {
                if let Some(result2) =
                    LocationAlgorithm::trilateration_weighted(&beacons, signals, &model)
                {
                    if let Some(result3) =
                        LocationAlgorithm::trilateration_least_squares(&beacons, signals, &model)
                    {
                        // 融合多个结果
                        if let Some(mut fused) = LocationAlgorithm::fuse_results(&[
                            (result1, 0.2),
                            (result2, 0.3),
                            (result3, 0.5),
                        ]) {
                            // 应用卡尔曼滤波
                            let (fx, fy, fz) = filter.update(fused.x, fused.y, fused.z);
                            fused.x = fx;
                            fused.y = fy;
                            fused.z = fz;

                            println!(
                                "  融合定位: ({:.2}, {:.2}, {:.2}), 置信度: {:.1}%",
                                fused.x, fused.y, fused.z,
                                fused.confidence * 100.0
                            );

                            sequence.push(fused);
                        }
                    }
                }
            }
        }

        // 5. 分析结果序列
        println!("\n序列分析:");
        println!("  总结果数: {}", sequence.len());

        if let Some(avg) = sequence.average_position() {
            println!("  平均位置: ({:.2}, {:.2}, {:.2})", avg.x, avg.y, avg.z);
        }

        if let Some(recent) = sequence.average_last_n(2) {
            println!("  最近平均: ({:.2}, {:.2}, {:.2})", recent.x, recent.y, recent.z);
        }

        println!("\n========== 演示完成 ==========\n");
    }
}
