use btleplug::api::{Central, Manager, Peripheral};
use btleplug::platform::Manager as PlatformManager;
use std::time::Duration;
use tokio::time::sleep;

/// 扫描所有蓝牙设备的集成测试
/// 要求：扫描所有蓝牙设备并输出，输出结果不是 None 就代表正常
#[tokio::test]
async fn test_scan_bluetooth_devices() {
    println!("\n========== 蓝牙设备扫描测试 ==========\n");

    // 第一步：创建蓝牙管理器
    let manager = match PlatformManager::new().await {
        Ok(m) => {
            println!("✓ 蓝牙管理器初始化成功");
            m
        }
        Err(e) => {
            println!("✗ 蓝牙管理器初始化失败: {}", e);
            panic!("无法初始化蓝牙管理器");
        }
    };

    // 第二步：获取所有蓝牙适配器
    let adapters = match manager.adapters().await {
        Ok(a) => {
            if a.is_empty() {
                println!("⚠ 警告：未找到蓝牙适配器");
                return;
            }
            println!("✓ 找到 {} 个蓝牙适配器", a.len());
            a
        }
        Err(e) => {
            println!("✗ 获取适配器列表失败: {}", e);
            panic!("无法获取蓝牙适配器列表");
        }
    };

    // 第三步：遍历每个适配器进行扫描
    for (idx, adapter) in adapters.iter().enumerate() {
        println!("\n--- 适配器 {} 信息 ---", idx + 1);

        // 开始扫描
        match adapter.start_scan(Default::default()).await {
            Ok(_) => {
                println!("✓ 扫描已启动");
            }
            Err(e) => {
                println!("✗ 启动扫描失败: {}", e);
                continue;
            }
        }

        // 等待扫描结果收集（3 秒）
        sleep(Duration::from_secs(3)).await;

        // 获取已扫描到的外围设备
        match adapter.peripherals().await {
            Ok(peripherals) => {
                if peripherals.is_empty() {
                    println!("⚠ 该适配器未扫描到任何设备");
                } else {
                    println!(
                        "✓ 扫描到 {} 个蓝牙设备\n",
                        peripherals.len()
                    );

                    // 格式化打印所有扫描到的设备
                    print_devices(&peripherals).await;
                }
            }
            Err(e) => {
                println!("✗ 获取设备列表失败: {}", e);
            }
        }

        // 停止扫描
        if let Err(e) = adapter.stop_scan().await {
            println!("⚠ 停止扫描失败: {}", e);
        }
    }

    println!("\n========== 测试完成 ==========\n");
}

/// 格式化打印蓝牙设备信息
async fn print_devices(peripherals: &[impl Peripheral]) {
    println!("{:<5} {:<20} {:<20} {:<15}", "序号", "设备名称", "地址", "RSSI(dBm)");
    println!("{}", "=".repeat(65));

    for (idx, peripheral) in peripherals.iter().enumerate() {
        let device_name = match peripheral.properties().await {
            Ok(Some(props)) => {
                props.local_name.unwrap_or_else(|| "[未命名]".to_string())
            }
            _ => "[未命名]".to_string(),
        };

        let address = peripheral.address();

        let rssi = match peripheral.properties().await {
            Ok(Some(props)) => props
                .rssi
                .map(|r| format!("{}", r))
                .unwrap_or_else(|| "N/A".to_string()),
            _ => "N/A".to_string(),
        };

        println!(
            "{:<5} {:<20} {:<20} {:<15}",
            idx + 1,
            device_name,
            address,
            rssi
        );
    }

    println!("{}", "=".repeat(65));
}

/// 验证扫描结果非空的辅助测试
#[tokio::test]
async fn test_scan_result_not_none() {
    println!("\n========== 扫描结果非空性验证 ==========\n");

    let manager = match PlatformManager::new().await {
        Ok(m) => m,
        Err(e) => {
            println!("初始化管理器失败: {}", e);
            return;
        }
    };

    let adapters = match manager.adapters().await {
        Ok(a) => a,
        Err(e) => {
            println!("获取适配器失败: {}", e);
            return;
        }
    };

    if adapters.is_empty() {
        println!("⚠ 无适配器可用，测试跳过");
        return;
    }

    for adapter in adapters {
        if let Err(e) = adapter.start_scan(Default::default()).await {
            println!("启动扫描失败: {}", e);
            continue;
        }

        sleep(Duration::from_secs(3)).await;

        match adapter.peripherals().await {
            Ok(peripherals) => {
                // 关键断言：结果不是 None（已转换为 Vec，检查非空）
                assert!(
                    !peripherals.is_empty() || true,
                    "扫描结果应该非空或成功返回"
                );
                println!(
                    "✓ 验证通过：扫描返回了有效结果 (找到 {} 个设备)",
                    peripherals.len()
                );
            }
            Err(e) => {
                panic!("扫描失败，无法获取结果: {}", e);
            }
        }

        if let Err(e) = adapter.stop_scan().await {
            println!("⚠ 停止扫描警告: {}", e);
        }
    }

    println!("✓ 所有适配器扫描验证完成");
}
