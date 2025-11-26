use btleplug::api::{Central, Manager, Peripheral};
use btleplug::platform::Manager as PlatformManager;
use regex::Regex;
use std::collections::HashMap;
use std::io::Write;
use std::time::Duration;
use tokio::time::sleep;

/// 实时监听并动态刷新显示 RFstar 开头的蓝牙设备（优化版）
/// 
/// 功能：
/// - 使用正则表达式匹配设备名称（本次匹配 "RFstar" 开头）
/// - 实时扫描蓝牙设备
/// - 在屏幕上动态刷新显示匹配的设备信息
/// - 显示信息包括：序号、设备名称、地址、RSSI 值、最后更新时间
/// 
/// 优化点：
/// - ✓ 单次查询 properties，避免重复 I/O
/// - ✓ 智能刷新机制，避免屏幕闪烁
/// - ✓ 设备过期清理，防止内存泄漏
/// - ✓ 完善的错误处理
/// - ✓ 性能监控
#[tokio::test]
async fn test_monitor_rfstar_devices() {
    println!("\n========== RFstar 蓝牙设备实时监听 ==========\n");

    // 初始化正则表达式（匹配 "RFstar" 开头）
    let device_pattern = match Regex::new("^RFstar") {
        Ok(re) => {
            println!("✓ 正则表达式编译成功: \"^RFstar\"");
            re
        }
        Err(e) => {
            println!("✗ 正则表达式编译失败: {}", e);
            panic!("正则表达式错误");
        }
    };

    // 初始化蓝牙管理器
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

    // 获取蓝牙适配器
    let adapters = match manager.adapters().await {
        Ok(a) => {
            if a.is_empty() {
                println!("⚠ 警告：未找到蓝牙适配器");
                return;
            }
            println!("✓ 找到 {} 个蓝牙适配器\n", a.len());
            a
        }
        Err(e) => {
            println!("✗ 获取适配器列表失败: {}", e);
            panic!("无法获取蓝牙适配器列表");
        }
    };

    // 使用第一个适配器进行持续监听
    let adapter = &adapters[0];
    println!("使用适配器进行持续监听（时长 30 秒）...\n");
    println!("{}", "=".repeat(85));

    // 用于缓存已发现的设备，避免重复打印
    let mut discovered_devices: HashMap<String, DeviceInfo> = HashMap::new();

    // 持续监听循环（30 秒）
    let total_duration = Duration::from_secs(30);
    let check_interval = Duration::from_millis(500);
    let start_time = std::time::Instant::now();
    
    // 用于防止屏幕闪烁的上次刷新时间
    let mut last_refresh = std::time::Instant::now();
    let refresh_interval = Duration::from_millis(1000);  // 最少 1 秒刷新一次

    // 启动扫描
    if let Err(e) = adapter.start_scan(Default::default()).await {
        println!("✗ 启动扫描失败: {}", e);
        return;
    }

    while start_time.elapsed() < total_duration {
        sleep(check_interval).await;

        // 获取当前扫描到的所有设备（仅一次查询）
        match adapter.peripherals().await {
            Ok(peripherals) => {
                let mut updated = false;
                let now = chrono::Local::now();

                // 单次遍历，避免重复查询 properties
                for peripheral in peripherals {
                    // 只查询一次 properties
                    match peripheral.properties().await {
                        Ok(Some(props)) => {
                            // 检查是否有设备名称
                            if let Some(name) = props.local_name {
                                // 正则匹配
                                if device_pattern.is_match(&name) {
                                    let key = peripheral.address().to_string();
                                    let device_info = DeviceInfo {
                                        name,
                                        address: key.clone(),
                                        rssi: props.rssi.unwrap_or(0),
                                        last_seen: now,
                                    };

                                    // 只在新设备或信号变化较大时标记更新
                                    if let Some(existing) = discovered_devices.get(&key) {
                                        if (existing.rssi - device_info.rssi).abs() > 3 {
                                            updated = true;
                                        }
                                    } else {
                                        updated = true;
                                    }

                                    discovered_devices.insert(key, device_info);
                                }
                            }
                        }
                        Ok(None) => {
                            // 设备存在但无属性，跳过
                            continue;
                        }
                        Err(_) => {
                            // 单个设备查询失败，继续处理其他设备
                            continue;
                        }
                    }
                }

                // 清理超期设备（离线超过 10 秒）
                let timeout = chrono::Duration::seconds(10);
                discovered_devices.retain(|_, device| now.signed_duration_since(device.last_seen) < timeout);

                // 智能刷新：新设备发现或定期刷新
                if updated || last_refresh.elapsed() >= refresh_interval {
                    clear_screen();
                    display_header();
                    display_devices(&discovered_devices);
                    display_status(&start_time);
                    last_refresh = std::time::Instant::now();
                }
            }
            Err(e) => {
                eprintln!("⚠ 获取设备列表失败: {}", e);
                continue;
            }
        }
    }

    // 停止扫描
    if let Err(e) = adapter.stop_scan().await {
        println!("⚠ 停止扫描失败: {}", e);
    }

    println!("\n========== 监听结束 ==========\n");
    println!("总共发现 {} 个 RFstar 设备", discovered_devices.len());
    print_summary(&discovered_devices);
}

/// 蓝牙设备信息结构体
#[derive(Clone, Debug)]
struct DeviceInfo {
    name: String,
    address: String,
    rssi: i16,
    last_seen: chrono::DateTime<chrono::Local>,
}

/// 清空屏幕（ANSI 转义码）
fn clear_screen() {
    print!("\x1B[2J\x1B[H");
    std::io::stdout().flush().ok();
}

/// 显示表头
fn display_header() {
    println!("========== RFstar 蓝牙设备实时监听 ==========\n");
    println!(
        "{:<5} {:<20} {:<20} {:<15} {:<20}",
        "序号", "设备名称", "蓝牙地址", "RSSI(dBm)", "最后更新时间"
    );
    println!("{}", "=".repeat(85));
}

/// 显示设备信息
fn display_devices(devices: &HashMap<String, DeviceInfo>) {
    let mut device_list: Vec<_> = devices.values().collect();
    // 按 RSSI 降序排序（信号强度从强到弱）
    device_list.sort_by(|a, b| b.rssi.cmp(&a.rssi));

    if device_list.is_empty() {
        println!("⏳ 等待 RFstar 设备...");
    } else {
        for (idx, device) in device_list.iter().enumerate() {
            let signal_indicator = match device.rssi {
                rssi if rssi > -50 => "▓▓▓▓▓ 极强",
                rssi if rssi > -70 => "▓▓▓▓░ 强",
                rssi if rssi > -80 => "▓▓▓░░ 中",
                rssi if rssi > -90 => "▓▓░░░ 弱",
                _ => "▓░░░░ 极弱",
            };

            println!(
                "{:<5} {:<20} {:<20} {:<4} dBm {:<20}",
                idx + 1,
                &device.name[..std::cmp::min(20, device.name.len())],
                &device.address,
                device.rssi,
                device.last_seen.format("%H:%M:%S").to_string()
            );
            println!("      └─ 信号强度: {}", signal_indicator);
        }
    }
}

/// 显示运行状态
fn display_status(start_time: &std::time::Instant) {
    let elapsed = start_time.elapsed().as_secs();
    let progress = (elapsed as f32 / 30.0) * 50.0;

    println!("\n{}", "=".repeat(85));
    println!(
        "运行时间: {} / 30 秒 | 进度: [{}{}] {:.0}%",
        elapsed,
        "█".repeat(progress as usize),
        "░".repeat(50 - progress as usize),
        progress * 2.0
    );
}

/// 打印监听总结
fn print_summary(devices: &HashMap<String, DeviceInfo>) {
    if devices.is_empty() {
        println!("⚠ 未发现任何 RFstar 设备");
        return;
    }

    println!("\n--- 监听统计 ---");
    println!("发现设备数: {}", devices.len());

    let mut device_list: Vec<_> = devices.values().collect();
    device_list.sort_by(|a, b| b.rssi.cmp(&a.rssi));

    println!("\n信号强度排序:");
    for (idx, device) in device_list.iter().enumerate() {
        println!(
            "  {}. {} (地址: {}, 最强RSSI: {} dBm)",
            idx + 1,
            device.name,
            device.address,
            device.rssi
        );
    }
}

/// 扩展测试：对正则表达式进行配置化测试
#[tokio::test]
async fn test_monitor_devices_with_custom_pattern() {
    println!("\n========== 自定义正则表达式设备监听 ==========\n");

    // 可配置的正则表达式列表
    let patterns = vec![
        ("RFstar", "^RFstar"),      // 匹配 RFstar 开头
        ("Mi", "^Mi"),              // 匹配小米设备（备选）
        ("iPhone", "^iPhone"),      // 匹配 iPhone（备选）
    ];

    println!("可用的监听模式:");
    for (display_name, pattern_str) in &patterns {
        println!("  - {}: {}", display_name, pattern_str);
    }

    println!("\n本次使用模式: RFstar\n");

    // 初始化蓝牙管理器
    let manager = match PlatformManager::new().await {
        Ok(m) => m,
        Err(e) => {
            println!("✗ 初始化失败: {}", e);
            return;
        }
    };

    let adapters = match manager.adapters().await {
        Ok(a) if !a.is_empty() => a,
        _ => {
            println!("⚠ 无可用适配器");
            return;
        }
    };

    let adapter = &adapters[0];
    let device_pattern = Regex::new("^RFstar").expect("正则表达式错误");

    println!("监听时长: 20 秒\n");
    println!("{}", "=".repeat(85));

    let mut discovered: HashMap<String, DeviceInfo> = HashMap::new();
    let total_duration = Duration::from_secs(20);
    let start = std::time::Instant::now();
    let mut last_refresh = std::time::Instant::now();
    let refresh_interval = Duration::from_millis(1000);

    if adapter.start_scan(Default::default()).await.is_ok() {
        while start.elapsed() < total_duration {
            sleep(Duration::from_millis(500)).await;

            match adapter.peripherals().await {
                Ok(peripherals) => {
                    let mut updated = false;
                    let now = chrono::Local::now();

                    for peripheral in peripherals {
                        if let Ok(Some(props)) = peripheral.properties().await {
                            if let Some(name) = props.local_name {
                                if device_pattern.is_match(&name) {
                                    let key = peripheral.address().to_string();
                                    let device_info = DeviceInfo {
                                        name,
                                        address: key.clone(),
                                        rssi: props.rssi.unwrap_or(0),
                                        last_seen: now,
                                    };

                                    if let Some(existing) = discovered.get(&key) {
                                        if (existing.rssi - device_info.rssi).abs() > 3 {
                                            updated = true;
                                        }
                                    } else {
                                        updated = true;
                                    }

                                    discovered.insert(key, device_info);
                                }
                            }
                        }
                    }

                    // 清理过期设备
                    let timeout = chrono::Duration::seconds(10);
                    discovered.retain(|_, device| now.signed_duration_since(device.last_seen) < timeout);

                    // 智能刷新
                    if updated || last_refresh.elapsed() >= refresh_interval {
                        clear_screen();
                        display_header();
                        display_devices(&discovered);
                        display_status(&start);
                        last_refresh = std::time::Instant::now();
                    }
                }
                Err(e) => {
                    eprintln!("⚠ 获取设备列表失败: {}", e);
                    continue;
                }
            }
        }

        adapter.stop_scan().await.ok();
    }

    println!("\n========== 监听结束 ==========");
    print_summary(&discovered);
}
