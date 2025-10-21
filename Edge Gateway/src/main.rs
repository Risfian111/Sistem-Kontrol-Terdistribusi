use rumqttc::{MqttOptions, AsyncClient, Event, Incoming, QoS};
use influxdb2::Client as InfluxClient;
use influxdb2::models::DataPoint;
use futures::stream;
use anyhow::Result;
use serde::Deserialize;
use std::time::Duration;
use tokio::time;
use tokio::task;
use tokio_util::codec::{FramedRead, LinesCodec};
use tokio_serial::SerialPortBuilderExt;
use futures::StreamExt;

// Struktur data yang diharapkan dari ESP32 (Suhu dan Kelembaban Penyimpanan)
#[derive(Debug, Deserialize)]
struct StorageTelemetry {
    // Diubah namanya agar lebih jelas
    temperature: f32, 
    humidity: f32,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Konstan untuk konfigurasi yang mudah diubah
    const THINGSBOARD_HOST: &str = "demo.thingsboard.io";
    const THINGSBOARD_TOKEN: &str = "vs4LHIbcEmNbVxxaB4EY"; // Ganti dengan token perangkat stroberi Anda
    const INFLUX_URL: &str = "http://localhost:8086";
    const INFLUX_ORG: &str = "strawberry_org"; // Organisasi InfluxDB yang disarankan
    const INFLUX_TOKEN: &str = "wv4n_sKUgQTt-uwoVIBwOGu4pdALo_5AMlJRQfxRPrkP4ZD5OSrRwZhnAANSu5584l0zhdRSgUQbdRNsiqjm7A=="; 
    const INFLUX_BUCKET: &str = "strawberry_storage_data"; // Bucket data stroberi
    const SERIAL_PORT_NAME: &str = "/dev/ttyACM0"; // Pastikan ini sesuai
    const SERIAL_BAUD_RATE: u32 = 115200;

    println!("🍓 Edge Gateway: Strawberry Storage Monitoring Started");

    // ---------------- MQTT Setup (ThingsBoard) ----------------
    let mut mqttoptions = MqttOptions::new("strawberry-edge-gateway", THINGSBOARD_HOST, 1883);
    mqttoptions.set_credentials(THINGSBOARD_TOKEN, "");
    mqttoptions.set_keep_alive(Duration::from_secs(30));
    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);
    println!("🔌 MQTT Client for ThingsBoard Initialized.");

    // ---------------- InfluxDB Setup ----------------
    let influx = InfluxClient::new(
        INFLUX_URL,
        INFLUX_ORG, 
        INFLUX_TOKEN, 
    );
    let bucket = INFLUX_BUCKET;
    println!("📊 InfluxDB Client Initialized. Target Bucket: {}", bucket);

    // ---------------- Serial Setup (Asynchronous) ----------------
    let serial = tokio_serial::new(SERIAL_PORT_NAME, SERIAL_BAUD_RATE)
        .timeout(Duration::from_secs(2))
        .open_native_async()
        .expect("❌ Gagal buka serial port. Cek koneksi dan nama port.");
    let mut reader = FramedRead::new(serial, LinesCodec::new());
    println!("🔗 Serial Port {} @ {} baud Opened.", SERIAL_PORT_NAME, SERIAL_BAUD_RATE);

    // ---------------- Task: MQTT Event Loop Handler ----------------
    task::spawn(async move {
        loop {
            match eventloop.poll().await {
                Ok(notification) => match &notification {
                    Event::Incoming(Incoming::ConnAck(_)) => {
                        println!("✅ ThingsBoard Connected!");
                    }
                    Event::Incoming(Incoming::Publish(p)) => {
                        // Menerima perintah dari ThingsBoard (misal: kontrol kipas override)
                        println!("📩 Command Received | Topic: {}, Payload: {:?}", p.topic, p.payload);
                    }
                    _ => {}, // Abaikan event MQTT lainnya
                },
                Err(e) => {
                    eprintln!("❌ MQTT eventloop error: {:?}", e);
                    break;
                }
            }
        }
    });

    // ---------------- Loop Pembacaan dan Pengiriman Data ----------------
    let mut data_counter: u64 = 1; 
    
    while let Some(line_result) = reader.next().await {
        match line_result {
            Ok(line) => {
                // Mencoba deserialisasi JSON yang diterima dari ESP32
                if let Ok(data) = serde_json::from_str::<StorageTelemetry>(&line) {
                    println!("\n─────── 📦 Data Cycle #{} ───────", data_counter);
                    println!("🌡️ Sensor Reading : Temp={:.1}°C, Hum={:.1}%", data.temperature, data.humidity);

                    // --- 1. Kirim ke ThingsBoard (MQTT) ---
                    let payload = format!(
                        r#"{{"temperature": {}, "humidity": {}}}"#,
                        data.temperature, data.humidity
                    );
                    match client
                        // Topik standar ThingsBoard untuk Telemetry
                        .publish("v1/devices/me/telemetry", QoS::AtLeastOnce, false, payload)
                        .await
                    {
                        Ok(_) => println!("⬆️ Sent to ThingsBoard via MQTT."),
                        Err(e) => eprintln!("❌ MQTT Publish Error: {:?}", e),
                    }

                    // --- 2. Simpan ke InfluxDB ---
                    let point = DataPoint::builder("storage_telemetry")
                        .tag("device", "strawberry-storage-unit") // Tagging data
                        .field("temperature", data.temperature as f64)
                        .field("humidity", data.humidity as f64)
                        .build()?;

                    if let Err(e) = influx.write(bucket, stream::iter(vec![point])).await {
                        eprintln!("❌ InfluxDB Write Error: {:?}", e);
                    } else {
                        println!("💾 Data successfully stored in InfluxDB.");
                    }

                    data_counter += 1;
                } else {
                    // Gagal parsing JSON
                    eprintln!("⚠️ Failed to parse JSON data: '{}'", line);
                }
            }
            Err(e) => {
                // Error saat membaca baris dari serial
                eprintln!("❌ Serial Read Error: {:?}", e);
            }
        }
        
        // Jeda singkat antar siklus pembacaan
        time::sleep(Duration::from_secs(1)).await;
    }

    Ok(())
}
