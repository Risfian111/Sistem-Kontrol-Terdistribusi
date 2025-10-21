# 🍓 Sistem Monitoring dan Kontrol Suhu Penyimpanan Stroberi Berbasis IoT

[cite_start]Proyek ini mengimplementasikan solusi **Internet of Things (IoT) *end-to-end*** yang efektif untuk manajemen kualitas buah **stroberi** (*perishable fruit*) pascapanen[cite: 17, 27, 2393, 2399]. [cite_start]Tujuannya adalah menjaga suhu dan kelembaban ruang penyimpanan agar tetap stabil sesuai standar kualitas stroberi ($0-10^{\circ}C$ dengan kelembaban 85-95%)[cite: 19, 89, 125].

[cite_start]Sistem ini didukung oleh arsitektur *Edge Computing* yang stabil menggunakan mikrokontroler **ESP32-S3** yang diprogram dengan bahasa **Rust**[cite: 74, 182, 2396]. [cite_start]Data diintegrasikan dengan *cloud platform* **ThingsBoard** untuk visualisasi dan **InfluxDB** untuk penyimpanan data *time-series*[cite: 25, 26, 69, 2321].

---

## 🏗️ Arsitektur dan Komponen Sistem

[cite_start]Arsitektur sistem ini dirancang modular, memisahkan fungsi akuisisi data, pemrosesan tepi (*edge processing*), dan visualisasi *cloud*[cite: 87, 88, 2404].

### 1. Lapisan Perangkat Keras (Edge Controller)

| Komponen | Peran dalam Sistem | Protokol Komunikasi |
| :--- | :--- | :--- |
| **ESP32-S3** | Mikrokontroler utama, berfungsi sebagai pusat kendali (*edge controller*). | [cite_start]MQTT [cite: 83, 91, 162] |
| **Sensor SHT20** | [cite_start]Mengukur suhu ($\pm0.3^{\circ}C$) dan kelembaban ($\pm2\%$ RH) lingkungan penyimpanan[cite: 53, 54, 128]. | [cite_start]Modbus RTU [cite: 55, 188] |
| **Aktuator** | [cite_start]**Kipas DC** (pendinginan) untuk menjaga suhu agar tidak melebihi batas aman ($10^{\circ}C$)[cite: 84, 123]. | [cite_start]GPIO Output [cite: 195] |

***

### 2. Embedded Software dan Edge Gateway

| Komponen | Bahasa / Teknologi | Peran Utama |
| :--- | :--- | :--- |
| **Firmware ESP32-S3** | [cite_start]Rust (dengan `esp-idf-svc`, `rmodbus`) [cite: 182] | [cite_start]Akuisisi data, serialisasi data ke JSON, dan menjalankan **Logika Kontrol Kipas Histeresis**[cite: 184, 201]. |
| **Edge Gateway** | [cite_start]Rust (runtime **Tokio**) [cite: 2321] | [cite_start]Menerima *stream* data JSON dari ESP32-S3 via **Port Serial**, memproses, dan meneruskan data ke ThingsBoard/InfluxDB[cite: 2321, 2333]. |
| **Komunikasi Cloud** | MQTT | [cite_start]Saluran komunikasi *real-time* yang ringan dari Edge Gateway ke ThingsBoard[cite: 162, 2397]. |

***

### 3. Logika Kontrol Kipas pada ESP32-S3

[cite_start]Logika kontrol diimplementasikan langsung pada *firmware* ESP32-S3 menggunakan konsep histeresis untuk mencegah *chattering*[cite: 201, 2398].

| Kondisi Suhu | Status Kipas | Durasi | Keterangan |
| :--- | :--- | :--- | :--- |
| [cite_start]Suhu $> 8.0^{\circ}C$ (**TEMP\_MAX**) [cite: 199] | Kipas ON | [cite_start]60 detik [cite: 202] | [cite_start]Suhu terlalu tinggi, pendingin diaktifkan, dan durasi pendinginan dicatat[cite: 202]. |
| [cite_start]Suhu $\le 6.0^{\circ}C$ (**TEMP\_MIN**) [cite: 199] | Kipas OFF | [cite_start]Segera [cite: 203] | [cite_start]Suhu sudah mencapai batas aman yang lebih rendah, kipas dimatikan[cite: 203]. |
| Kelembaban $< 90.0\%$ | Warning | - | [cite_start]Dicatat sebagai peringatan (*warning*) karena berpotensi cepat layu[cite: 200, 314]. |

***

### 4. Cloud Platform (Penyimpanan & Visualisasi)

| Platform | Tujuan | Format Data |
| :--- | :--- | :--- |
| **ThingsBoard** | [cite_start]Dashboard visualisasi data *real-time* (grafik, indikator) dan menerima perintah kontrol *override*[cite: 69, 164, 166]. | JSON (Telemetry) |
| **InfluxDB** | [cite_start]Penyimpanan data *time-series* untuk analisis historis dan pelaporan[cite: 2327, 2401]. | [cite_start]DataPoint [cite: 2387] |

---

## ✨ Potensi Pengembangan (Saran)

[cite_start]Pengembangan selanjutnya disarankan untuk meningkatkan keterandalan dan fungsionalitas sistem[cite: 2406]:

* [cite_start]**Aktuator Kelembaban:** Menambahkan aktuator (misalnya, *humidifier*) dan memperluas logika kontrol pada ESP32-S3 untuk menjaga kelembaban ($90-95\%$)[cite: 2406, 2407].
* [cite_start]**Kontrol *Override* Dua Arah:** Memastikan implementasi penuh downlink dari ThingsBoard ke Edge Gateway, memungkinkan pengguna mengontrol aktuator secara manual[cite: 2408].
* [cite_start]***Store-and-Forward* Gateway:** Mengimplementasikan penyimpanan data lokal di Edge Gateway untuk mencegah kehilangan data jika koneksi *cloud* terputus, dan mengirimkannya ulang setelah koneksi pulih[cite: 2409, 2410].
* [cite_start]**Analisis Prediktif:** Memanfaatkan data InfluxDB untuk analisis prediktif dan sistem notifikasi alarm yang lebih canggih (misalnya, via email atau aplikasi *mobile*)[cite: 2411].
