use esp_idf_svc::hal as hal;
use hal::{
    gpio::*,
    peripherals::Peripherals,
    prelude::*,
    uart::*,
};
use hal::uart::config::Config as UartConfig;

use rmodbus::{client::ModbusRequest, ModbusProto};
use serde::Serialize;
use std::{
    thread,
    time::{Duration, Instant},
};

// Data yang akan dikirim, mencerminkan pemantauan penyimpanan
#[derive(Serialize)]
struct Sample {
    temperature: f32,
    humidity: f32,
}

fn main() -> anyhow::Result<()> {
    // Inisialisasi sistem dan logger
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let p = Peripherals::take().unwrap();

    // ================= MAX485 / SHT20 (Modbus RTU) =================
    // Diasumsikan SHT20 terhubung melalui Modbus RTU ke ESP32.
    // Jika SHT20 terhubung I2C, bagian ini harus diganti dengan driver I2C.
    let mut de_re = PinDriver::output(p.pins.gpio4)?; // DE/RE MAX485
    let config = UartConfig::default().baudrate(Hertz(9600));
    let uart = UartDriver::new(
        p.uart1,
        p.pins.gpio18, // TX → DI MAX485
        p.pins.gpio17, // RX ← RO MAX485
        Option::<AnyIOPin>::None,
        Option::<AnyIOPin>::None,
        &config,
    )?;

    // ================= L9110 FAN (Aktor Pendingin) =================
    // Mengontrol kipas sebagai aktuator pendingin
    let mut fan_in_a = PinDriver::output(p.pins.gpio15)?; // IN A
    let mut fan_in_b = PinDriver::output(p.pins.gpio16)?; // IN B
    fan_in_a.set_low()?;
    fan_in_b.set_low()?;
    log::info!("❄️ Fan/Cooling actuator initialized (OFF)");

    // ================= LOOP KONTROL =================
    log::info!("🍓 Strawberry Storage Monitoring & Control System Started");

    let mut fan_on_until: Option<Instant> = None;
    
    // Suhu batas atas ideal untuk stroberi
    const TEMP_MAX: f32 = 8.0; 
    // Suhu batas bawah untuk mematikan kipas setelah pendinginan
    const TEMP_MIN: f32 = 6.0; 
    const COOLING_DURATION_SECS: u64 = 60; // Durasi kipas menyala per siklus

    loop {
        if let (Some(t), Some(h)) = (
            // Asumsi: 0x0001 = Suhu, 0x0002 = Kelembaban
            read_input_register(&uart, &mut de_re, 1, 0x0001), 
            read_input_register(&uart, &mut de_re, 1, 0x0002),
        ) {
            let sample = Sample { temperature: t, humidity: h };
            // Cetak data untuk dikirim ke Edge Gateway/MQTT (sesuai kode asli)
            println!("{}", serde_json::to_string(&sample).unwrap());
            log::info!("✅ Data Sensor OK: Temp={:.1}°C, Hum={:.1}%", t, h);

            // --- KONTROL PENDINGINAN KIPAS ---
            if t > TEMP_MAX {
                // Suhu terlalu tinggi, nyalakan pendingin
                if fan_on_until.is_none() {
                    fan_in_a.set_low()?;
                    fan_in_b.set_high()?; // Asumsi: Konfigurasi ini menyalakan kipas
                    fan_on_until = Some(Instant::now() + Duration::from_secs(COOLING_DURATION_SECS));
                    log::warn!("🌡️ Suhu {:.1}°C > {}°C → Kipas ON (Pendinginan {} detik)", t, TEMP_MAX, COOLING_DURATION_SECS);
                }
            } else if t <= TEMP_MIN {
                // Suhu sudah mencapai batas aman, matikan kipas segera
                 if fan_on_until.is_some() {
                    fan_in_a.set_low()?;
                    fan_in_b.set_low()?;
                    fan_on_until = None;
                    log::info!("✅ Suhu {:.1}°C ≤ {}°C → Kipas OFF (Tujuan tercapai)", t, TEMP_MIN);
                }
            }

            // Matikan kipas jika waktu ON habis (hanya berlaku jika target belum tercapai)
            if let Some(end_time) = fan_on_until {
                if Instant::now() >= end_time {
                    fan_in_a.set_low()?;
                    fan_in_b.set_low()?;
                    fan_on_until = None;
                    log::info!("⏳ Kipas OFF (Waktu pendinginan {} detik habis)", COOLING_DURATION_SECS);
                }
            }
            
            // --- KONTROL KELEMBAPAN (Hanya Monitoring/Alarm) ---
            if h < 90.0 {
                 log::warn!("⚠️ Kelembaban {:.1}% di bawah 90%! Stroberi berpotensi cepat layu.", h);
            }

        } else {
            log::warn!("❌ Gagal baca data sensor SHT20 (Modbus) — coba lagi...");
        }

        // Delay sebelum loop berikutnya
        thread::sleep(Duration::from_secs(30));
    }
}

/// Fungsi baca 1 register input (function 0x04)
/// (Fungsi ini tidak diubah karena logika Modbus tetap)
fn read_input_register(
    uart: &UartDriver,
    de_re: &mut PinDriver<'_, Gpio4, Output>,
    unit_id: u8,
    register: u16,
) -> Option<f32> {
    let mut mreq = ModbusRequest::new(unit_id, ModbusProto::Rtu);
    let mut txbuf: Vec<u8> = Vec::with_capacity(256);
    if mreq.generate_get_inputs(register, 1, &mut txbuf).is_err() {
        log::error!("❌ generate_get_inputs failed for 0x{:04X}", register);
        return None;
    }

    // Transmit
    let _ = de_re.set_high();
    let _ = uart.write(&txbuf);
    let _ = uart.wait_tx_done(100);
    let _ = de_re.set_low();

    // Receive
    let mut rxbuf = vec![0u8; 512];
    let n = match uart.read(&mut rxbuf, 500) {
        Ok(n) if n > 0 => n,
        _ => return None,
    };

    // Parse and return value (assuming value is scaled by 10.0)
    let mut vals = Vec::new();
    if mreq.parse_u16(&rxbuf[..n], &mut vals).is_ok() && !vals.is_empty() {
        Some(vals[0] as f32 / 10.0)
    } else {
        None
    }
}

// Fungsi servo_duty Dihapus karena servo tidak digunakan.