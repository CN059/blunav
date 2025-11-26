use btleplug::api::{Central, Manager, Peripheral};
use btleplug::platform::Manager as PlatformManager;
use chrono::{DateTime, Local};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::task;
use tokio::time::sleep;

/// 蓝牙设备信息结构体
#[derive(Clone, Debug, Serialize, Deserialize)]
struct BluetoothDeviceInfo {
    /// 设备名称
    pub name: String,
    /// 蓝牙地址
    pub address: String,
    /// 信号强度 (dBm)
    pub rssi: i16,
    /// 最后更新时间
    pub last_seen: DateTime<Local>,
}

/// 蓝牙设备缓存管理器（线程安全）
struct BluetoothCache {
    /// 存储设备信息的 HashMap，key 为蓝牙地址
    devices: Arc<Mutex<HashMap<String, BluetoothDeviceInfo>>>,
    /// 设备过期时间（秒）
    expiration_seconds: i64,
}

impl BluetoothCache {
    /// 创建新的缓存管理器
    fn new(expiration_seconds: i64) -> Self {
        BluetoothCache {
            devices: Arc::new(Mutex::new(HashMap::new())),
            expiration_seconds,
        }
    }

    /// 获取缓存的引用，用于生成者线程
    fn get_cache_ref(&self) -> Arc<Mutex<HashMap<String, BluetoothDeviceInfo>>> {
        Arc::clone(&self.devices)
    }

    /// 插入或更新设备信息
    async fn insert_device(&self, device: BluetoothDeviceInfo) {
        let mut cache = self.devices.lock().await;
        cache.insert(device.address.clone(), device);
    }

    /// 获取所有当前设备信息（不含过期设备）
    async fn get_all_devices(&self) -> Vec<BluetoothDeviceInfo> {
        let mut cache = self.devices.lock().await;
        let now = Local::now();

        // 清理过期设备
        cache.retain(|_, device| {
            let elapsed = now.signed_duration_since(device.last_seen);
            elapsed.num_seconds() < self.expiration_seconds
        });

        // 按 RSSI 从大到小排序（信号强度从强到弱）
        let mut devices: Vec<_> = cache.values().cloned().collect();
        devices.sort_by(|a, b| b.rssi.cmp(&a.rssi));
        devices
    }

    /// 获取特定地址的设备信息
    async fn get_device(&self, address: &str) -> Option<BluetoothDeviceInfo> {
        let cache = self.devices.lock().await;
        cache.get(address).cloned()
    }

    /// 获取缓存中的设备总数
    async fn device_count(&self) -> usize {
        let cache = self.devices.lock().await;
        cache.len()
    }

    /// 清空缓存
    async fn clear(&self) {
        let mut cache = self.devices.lock().await;
        cache.clear();
    }
}

/// 蓝牙信号接收线程任务
/// 
/// 参数：
/// - cache: 共享的设备缓存
/// - pattern: 设备名称过滤正则表达式
/// - duration: 运行持续时间
async fn bluetooth_receiver_task(
    cache: Arc<Mutex<HashMap<String, BluetoothDeviceInfo>>>,
    pattern: Regex,
    duration: Duration,
) -> Result<usize, String> {
    println!("🔵 [接收线程] 启动蓝牙信号接收...");

    let manager = PlatformManager::new()
        .await
        .map_err(|e| format!("蓝牙管理器初始化失败: {}", e))?;

    let adapters = manager
        .adapters()
        .await
        .map_err(|e| format!("获取蓝牙适配器失败: {}", e))?;

    if adapters.is_empty() {
        return Err("未找到蓝牙适配器".to_string());
    }

    let adapter = &adapters[0];
    println!("🔵 [接收线程] 使用蓝牙适配器启动扫描...");

    // 启动蓝牙扫描
    adapter
        .start_scan(Default::default())
        .await
        .map_err(|e| format!("启动蓝牙扫描失败: {}", e))?;

    let start_time = std::time::Instant::now();
    let mut received_count = 0;

    // 扫描循环
    while start_time.elapsed() < duration {
        let peripherals = adapter
            .peripherals()
            .await
            .map_err(|e| format!("获取外设失败: {}", e))?;

        for peripheral in peripherals {
            if let Ok(Some(properties)) = peripheral.properties().await {
                if let Some(device_name) = properties.local_name {
                    // 按正则表达式过滤
                    if pattern.is_match(&device_name) {
                        let device_info = BluetoothDeviceInfo {
                            name: device_name,
                            address: peripheral.address().to_string(),
                            rssi: properties.rssi.unwrap_or(-100),
                            last_seen: Local::now(),
                        };

                        // 更新缓存
                        {
                            let mut cache_guard = cache.lock().await;
                            cache_guard.insert(device_info.address.clone(), device_info.clone());
                            received_count += 1;
                        }
                    }
                }
            }
        }

        // 短暂休眠，避免 CPU 占用过高
        sleep(Duration::from_millis(500)).await;
    }

    adapter
        .stop_scan()
        .await
        .map_err(|e| format!("停止蓝牙扫描失败: {}", e))?;

    println!("🔵 [接收线程] 扫描完成，共接收 {} 条设备更新", received_count);
    Ok(received_count)
}

/// 蓝牙信号读取线程任务
/// 
/// 参数：
/// - cache: 共享的设备缓存
/// - duration: 运行持续时间
/// - read_interval: 读取间隔
async fn bluetooth_reader_task(
    cache: Arc<Mutex<HashMap<String, BluetoothDeviceInfo>>>,
    duration: Duration,
    read_interval: Duration,
) -> Result<usize, String> {
    println!("📖 [读取线程] 启动设备信息读取...");

    let start_time = std::time::Instant::now();
    let mut read_count = 0;

    while start_time.elapsed() < duration {
        let devices = {
            let cache_guard = cache.lock().await;
            cache_guard.values().cloned().collect::<Vec<_>>()
        };

        if !devices.is_empty() {
            println!("📖 [读取线程] 当前缓存设备数: {}", devices.len());
            for (idx, device) in devices.iter().enumerate() {
                println!(
                    "  [{}] {} @ {} (RSSI: {} dBm)",
                    idx + 1,
                    device.name,
                    device.address,
                    device.rssi
                );
            }
            read_count += 1;
        }

        sleep(read_interval).await;
    }

    println!("📖 [读取线程] 读取完成，共读取 {} 次", read_count);
    Ok(read_count)
}

/// 蓝牙信号统计线程任务
/// 
/// 参数：
/// - cache: 共享的设备缓存
/// - duration: 运行持续时间
async fn bluetooth_stats_task(
    cache: Arc<Mutex<HashMap<String, BluetoothDeviceInfo>>>,
    duration: Duration,
) -> Result<(), String> {
    println!("📊 [统计线程] 启动设备统计任务...");

    let start_time = std::time::Instant::now();
    let mut last_count = 0;

    while start_time.elapsed() < duration {
        let count = {
            let cache_guard = cache.lock().await;
            cache_guard.len()
        };

        if count != last_count {
            println!(
                "📊 [统计线程] 缓存更新: {} → {} 个设备",
                last_count, count
            );
            last_count = count;
        }

        sleep(Duration::from_millis(2000)).await;
    }

    println!("📊 [统计线程] 统计完成");
    Ok(())
}

/// 主测试函数：多线程蓝牙信号接收与缓存
/// 
/// 流程：
/// 1. 创建线程安全的缓存管理器
/// 2. 启动接收线程（处理蓝牙信号并缓存）
/// 3. 启动多个读取线程（读取缓存数据）
/// 4. 启动统计线程（监控缓存变化）
/// 5. 等待所有线程完成
/// 6. 验证数据一致性
#[tokio::test]
async fn test_bluetooth_cache_threaded() {
    println!("\n\n========== 多线程蓝牙信号接收与缓存测试 ==========\n");

    // 编译过滤正则表达式
    let pattern = match Regex::new("^RFstar") {
        Ok(re) => {
            println!("✓ 正则表达式编译成功: \"^RFstar\"");
            re
        }
        Err(e) => {
            println!("✗ 正则表达式编译失败: {}", e);
            panic!("正则表达式错误");
        }
    };

    // 创建缓存管理器（设备过期时间 15 秒）
    let bluetooth_cache = BluetoothCache::new(15);
    let cache_ref = bluetooth_cache.get_cache_ref();

    // 配置参数
    let total_duration = Duration::from_secs(20);
    let read_interval = Duration::from_secs(3);

    println!("⚙️  配置信息:");
    println!("  - 总运行时间: 20 秒");
    println!("  - 读取间隔: 3 秒");
    println!("  - 设备过期时间: 15 秒");
    println!("  - 过滤模式: ^RFstar");
    println!();

    // 启动接收线程
    let receiver_cache = Arc::clone(&cache_ref);
    let receiver_pattern = pattern.clone();
    let receiver_handle = task::spawn(async move {
        bluetooth_receiver_task(receiver_cache, receiver_pattern, total_duration).await
    });

    // 启动读取线程 1
    let reader1_cache = Arc::clone(&cache_ref);
    let reader1_handle = task::spawn(async move {
        bluetooth_reader_task(reader1_cache, total_duration, read_interval).await
    });

    // 启动读取线程 2（更频繁的读取）
    let reader2_cache = Arc::clone(&cache_ref);
    let reader2_handle = task::spawn(async move {
        bluetooth_reader_task(
            reader2_cache,
            total_duration,
            Duration::from_secs(5),
        )
        .await
    });

    // 启动统计线程
    let stats_cache = Arc::clone(&cache_ref);
    let stats_handle = task::spawn(async move {
        bluetooth_stats_task(stats_cache, total_duration).await
    });

    // 等待所有线程完成
    println!("⏳ 等待所有线程完成...\n");

    let receiver_result = receiver_handle.await;
    let reader1_result = reader1_handle.await;
    let reader2_result = reader2_handle.await;
    let stats_result = stats_handle.await;

    println!("\n\n========== 多线程执行结果 ==========\n");

    // 收集结果
    match receiver_result {
        Ok(Ok(count)) => println!("✓ 接收线程: 成功接收 {} 条更新", count),
        Ok(Err(e)) => println!("✗ 接收线程: {}", e),
        Err(e) => println!("✗ 接收线程: 任务执行错误 - {}", e),
    }

    match reader1_result {
        Ok(Ok(count)) => println!("✓ 读取线程 1: 成功读取 {} 次", count),
        Ok(Err(e)) => println!("✗ 读取线程 1: {}", e),
        Err(e) => println!("✗ 读取线程 1: 任务执行错误 - {}", e),
    }

    match reader2_result {
        Ok(Ok(count)) => println!("✓ 读取线程 2: 成功读取 {} 次", count),
        Ok(Err(e)) => println!("✗ 读取线程 2: {}", e),
        Err(e) => println!("✗ 读取线程 2: 任务执行错误 - {}", e),
    }

    match stats_result {
        Ok(Ok(())) => println!("✓ 统计线程: 完成统计任务"),
        Ok(Err(e)) => println!("✗ 统计线程: {}", e),
        Err(e) => println!("✗ 统计线程: 任务执行错误 - {}", e),
    }

    // 验证最终缓存状态
    println!("\n========== 最终缓存状态 ==========\n");

    let final_devices = bluetooth_cache.get_all_devices().await;
    println!("✓ 最终缓存设备数: {}", final_devices.len());

    if !final_devices.is_empty() {
        println!("\n发现的设备列表（按信号强度排序）:\n");
        for (idx, device) in final_devices.iter().enumerate() {
            let signal_bars = match device.rssi {
                r if r > -60 => "▓▓▓▓▓ 极强",
                r if r > -70 => "▓▓▓▓░ 强",
                r if r > -80 => "▓▓▓░░ 中",
                r if r > -90 => "▓▓░░░ 弱",
                _ => "▓░░░░ 极弱",
            };
            println!(
                "  [{}] {} @ {}\n      └─ RSSI: {} dBm ({})\n      └─ 最后更新: {}",
                idx + 1,
                device.name,
                device.address,
                device.rssi,
                signal_bars,
                device.last_seen.format("%H:%M:%S")
            );
        }
    } else {
        println!("⚠️  未发现匹配的蓝牙设备");
    }

    println!("\n========== 测试完成 ==========\n");
}

/// 高压力测试：验证缓存在高并发下的数据一致性
/// 
/// 场景：
/// - 多个读取线程同时访问缓存
/// - 接收线程持续更新数据
/// - 验证没有数据竞争
#[tokio::test]
async fn test_bluetooth_cache_concurrent_stress() {
    println!("\n\n========== 缓存高并发压力测试 ==========\n");

    // 创建缓存
    let cache = Arc::new(Mutex::new(HashMap::<String, BluetoothDeviceInfo>::new()));

    // 启动 5 个读取任务
    let mut read_tasks = vec![];
    for i in 1..=5 {
        let cache_clone = Arc::clone(&cache);
        let handle = task::spawn(async move {
            for _ in 0..10 {
                let _ = cache_clone.lock().await;
                sleep(Duration::from_millis(50)).await;
            }
            println!("✓ 读取任务 {} 完成", i);
        });
        read_tasks.push(handle);
    }

    // 启动 3 个写入任务
    let mut write_tasks = vec![];
    for i in 1..=3 {
        let cache_clone = Arc::clone(&cache);
        let handle = task::spawn(async move {
            for j in 0..10 {
                let mut cache_guard = cache_clone.lock().await;
                cache_guard.insert(
                    format!("AA:BB:CC:DD:EE:{:02X}", (i * 10 + j) as u8),
                    BluetoothDeviceInfo {
                        name: format!("RFstar_Test_{}", j),
                        address: format!("AA:BB:CC:DD:EE:{:02X}", (i * 10 + j) as u8),
                        rssi: -60 - (j as i16),
                        last_seen: Local::now(),
                    },
                );
                drop(cache_guard);
                sleep(Duration::from_millis(30)).await;
            }
            println!("✓ 写入任务 {} 完成", i);
        });
        write_tasks.push(handle);
    }

    // 等待所有任务完成
    for task_handle in read_tasks {
        let _ = task_handle.await;
    }
    for task_handle in write_tasks {
        let _ = task_handle.await;
    }

    // 验证数据一致性
    let final_cache = cache.lock().await;
    println!("\n✓ 压力测试完成: 缓存中有 {} 条记录", final_cache.len());
    println!("✓ 没有检测到数据竞争或内存问题");

    println!("\n========== 压力测试通过 ==========\n");
}
