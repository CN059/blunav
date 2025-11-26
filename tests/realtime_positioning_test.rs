/// 🎯 实时定位系统
/// 
/// 功能：
/// - 持续接收蓝牙信号
/// - 实时计算设备坐标
/// - 多线程架构，高效处理
/// - 清晰的命令行输出
/// 
/// 信标配置：
/// - C5D6: (764, 216, 63) cm
/// - 0CF1: (0, 152, 157) cm
/// - FBFC: (309, 748, 63) cm
/// 
/// RSSI 模型：
/// - A = -49.656 dBm
/// - B = -43.284
/// - n = 4.328

use blunav::positioning::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration, Instant};
use chrono::Local;

#[derive(Clone, Debug)]
struct SignalReading {
    beacon_address: String,
    beacon_name: String,
    rssi: i16,
    timestamp: chrono::DateTime<Local>,
}

struct PositioningConfig {
    beacons: HashMap<String, Beacon>,
    rssi_model: RSSIModel,
    update_interval: Duration,
    kalman: Arc<Mutex<KalmanFilter>>,
}

impl PositioningConfig {
    fn new() -> Self {
        let mut beacons = HashMap::new();

        beacons.insert(
            "20:A7:16:5E:C5:D6".to_string(),
            Beacon {
                id: "20:A7:16:5E:C5:D6".to_string(),
                name: "RFstar_C5D6".to_string(),
                x: 764.0,
                y: 216.0,
                z: 63.0,
            },
        );

        beacons.insert(
            "20:A7:16:61:0C:F1".to_string(),
            Beacon {
                id: "20:A7:16:61:0C:F1".to_string(),
                name: "RFstar_0CF1".to_string(),
                x: 0.0,
                y: 152.0,
                z: 157.0,
            },
        );

        beacons.insert(
            "20:A7:16:60:FB:FC".to_string(),
            Beacon {
                id: "20:A7:16:60:FB:FC".to_string(),
                name: "RFstar_FBFC".to_string(),
                x: 309.0,
                y: 748.0,
                z: 63.0,
            },
        );

        let rssi_model = RSSIModel::new(-49.656, -43.284, 4.328);
        let kalman = KalmanFilter::new(400.0, 400.0);

        PositioningConfig {
            beacons,
            rssi_model,
            update_interval: Duration::from_millis(500),
            kalman: Arc::new(Mutex::new(kalman)),
        }
    }
}

fn format_signal_level(rssi: i16) -> String {
    match rssi {
        r if r > -50 => "▓▓▓▓▓ 极强".to_string(),
        r if r > -60 => "▓▓▓▓░ 强".to_string(),
        r if r > -70 => "▓▓▓░░ 中".to_string(),
        r if r > -80 => "▓▓░░░ 弱".to_string(),
        _ => "▓░░░░ 极弱".to_string(),
    }
}

fn print_location_result(
    result_no: usize,
    readings: &HashMap<String, SignalReading>,
    x: f64,
    y: f64,
    z: f64,
    confidence: f64,
    error: f64,
    method: &str,
    elapsed: Duration,
) {
    let elapsed_secs = elapsed.as_secs();
    let elapsed_millis = elapsed.subsec_millis();

    println!("📍 定位结果 #{} | 运行时间: {}s {}ms", result_no, elapsed_secs, elapsed_millis);
    println!("┌─ 位置坐标 (cm)");
    println!("│  X: {:>8.2} cm", x);
    println!("│  Y: {:>8.2} cm", y);
    println!("│  Z: {:>8.2} cm", z);
    println!("├─ 定位质量");
    println!("│  方法: {}", method);
    println!("│  置信度: {:>6.1}%", confidence * 100.0);
    println!("│  误差: {:>7.2} cm", error);
    println!("├─ 信号信息");

    for (addr, reading) in readings {
        let signal_level = format_signal_level(reading.rssi);
        println!(
            "│  {} ({}): {} dBm {}",
            reading.beacon_name, addr, reading.rssi, signal_level
        );
    }

    println!("└─ 时间: {}", Local::now().format("%H:%M:%S%.3f"));
}

async fn realtime_positioning_task(
    config: Arc<PositioningConfig>,
    mut signal_rx: tokio::sync::mpsc::Receiver<SignalReading>,
) {
    println!("\n🎯 [定位线程] 启动实时定位计算...\n");

    let mut latest_readings: HashMap<String, SignalReading> = HashMap::new();
    let mut result_count = 0;
    let start_time = Instant::now();

    loop {
        match tokio::time::timeout(
            Duration::from_secs(1),
            signal_rx.recv(),
        )
        .await
        {
            Ok(Some(reading)) => {
                latest_readings.insert(reading.beacon_address.clone(), reading.clone());
            }
            Ok(None) => {
                println!("📡 [定位线程] 信号接收通道关闭");
                break;
            }
            Err(_) => {
                // 超时，继续处理现有数据
            }
        }

        if latest_readings.len() >= 3 {
            let mut beacons_with_distances = Vec::new();

            for (addr, reading) in &latest_readings {
                if let Some(beacon) = config.beacons.get(addr) {
                    let distance = config.rssi_model.rssi_to_distance(reading.rssi);
                    beacons_with_distances.push((
                        beacon.x,
                        beacon.y,
                        beacon.z,
                        distance,
                    ));
                }
            }

            if beacons_with_distances.len() >= 3 {
                if let Some(raw_result) = trilateration_least_squares(&beacons_with_distances) {
                    let mut kalman = config.kalman.lock().await;
                    kalman.update(raw_result.x, raw_result.y, 0.5);
                    let (filtered_x, filtered_y) = kalman.position();

                    result_count += 1;

                    if result_count % 2 == 1 {
                        println!("{}", "═".repeat(88));
                    }

                    print_location_result(
                        result_count,
                        &latest_readings,
                        filtered_x,
                        filtered_y,
                        raw_result.z,
                        raw_result.confidence,
                        raw_result.error,
                        &raw_result.method,
                        start_time.elapsed(),
                    );
                }
            }
        }

        sleep(config.update_interval).await;
    }

    println!("\n✓ 定位线程已停止");
}

async fn simulated_signal_source(
    tx: tokio::sync::mpsc::Sender<SignalReading>,
) {
    println!("📡 [信号线程] 启动模拟蓝牙信号源...\n");

    let signal_sequences = vec![
        vec![
            SignalReading {
                beacon_address: "20:A7:16:5E:C5:D6".to_string(),
                beacon_name: "RFstar_C5D6".to_string(),
                rssi: -52,
                timestamp: Local::now(),
            },
            SignalReading {
                beacon_address: "20:A7:16:61:0C:F1".to_string(),
                beacon_name: "RFstar_0CF1".to_string(),
                rssi: -77,
                timestamp: Local::now(),
            },
            SignalReading {
                beacon_address: "20:A7:16:60:FB:FC".to_string(),
                beacon_name: "RFstar_FBFC".to_string(),
                rssi: -86,
                timestamp: Local::now(),
            },
        ],
        vec![
            SignalReading {
                beacon_address: "20:A7:16:5E:C5:D6".to_string(),
                beacon_name: "RFstar_C5D6".to_string(),
                rssi: -48,
                timestamp: Local::now(),
            },
            SignalReading {
                beacon_address: "20:A7:16:61:0C:F1".to_string(),
                beacon_name: "RFstar_0CF1".to_string(),
                rssi: -70,
                timestamp: Local::now(),
            },
            SignalReading {
                beacon_address: "20:A7:16:60:FB:FC".to_string(),
                beacon_name: "RFstar_FBFC".to_string(),
                rssi: -80,
                timestamp: Local::now(),
            },
        ],
        vec![
            SignalReading {
                beacon_address: "20:A7:16:5E:C5:D6".to_string(),
                beacon_name: "RFstar_C5D6".to_string(),
                rssi: -65,
                timestamp: Local::now(),
            },
            SignalReading {
                beacon_address: "20:A7:16:61:0C:F1".to_string(),
                beacon_name: "RFstar_0CF1".to_string(),
                rssi: -68,
                timestamp: Local::now(),
            },
            SignalReading {
                beacon_address: "20:A7:16:60:FB:FC".to_string(),
                beacon_name: "RFstar_FBFC".to_string(),
                rssi: -50,
                timestamp: Local::now(),
            },
        ],
    ];

    let mut iteration = 0;
    loop {
        for signals in &signal_sequences {
            for signal in signals {
                let mut signal = signal.clone();
                signal.timestamp = Local::now();
                let _ = tx.send(signal).await;
                sleep(Duration::from_millis(100)).await;
            }
        }

        iteration += 1;
        if iteration >= 3 {
            break;
        }
    }

    println!("\n📡 [信号线程] 信号序列发送完成");
}

#[tokio::test]
async fn test_realtime_positioning() {
    println!("\n\n");
    println!("╔══════════════════════════════════════════════════════════════════════════════════╗");
    println!("║                    🎯 实时蓝牙室内定位系统                                      ║");
    println!("╚══════════════════════════════════════════════════════════════════════════════════╝");

    let config = Arc::new(PositioningConfig::new());

    println!("\n📋 系统配置信息:");
    println!("├─ 信标配置:");
    for (addr, beacon) in &config.beacons {
        println!(
            "│  {} ({})",
            beacon.name, addr
        );
        println!(
            "│    位置: ({:.1}, {:.1}, {:.1}) cm",
            beacon.x, beacon.y, beacon.z
        );
    }

    println!("├─ RSSI 转距离模型:");
    println!("│  公式: RSSI(d) = A + B * log₁₀(d)");
    println!("│  参数: A = -49.656 dBm, B = -43.284, n = 4.328");
    println!("├─ 定位更新间隔: {:.0} ms", config.update_interval.as_millis());
    println!("└─ 使用算法: 最小二乘法 + 卡尔曼滤波\n");

    let (tx, rx) = tokio::sync::mpsc::channel(100);

    let signal_task = tokio::spawn(async move {
        simulated_signal_source(tx).await;
    });

    let config_clone = Arc::clone(&config);
    let positioning_task = tokio::spawn(async move {
        realtime_positioning_task(config_clone, rx).await;
    });

    let _ = tokio::join!(signal_task, positioning_task);

    println!("\n╔══════════════════════════════════════════════════════════════════════════════════╗");
    println!("║                        ✓ 测试完成                                              ║");
    println!("╚══════════════════════════════════════════════════════════════════════════════════╝\n");
}
