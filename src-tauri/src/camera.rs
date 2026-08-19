//! Optionale Netzwerk-Kamera (z. B. ein Handy als Webcam per MJPEG).
//!
//! Warum der Umweg über Rust? Ein MJPEG-Stream von einer fremden Adresse ist
//! für den Webview eine Cross-Origin-Bildquelle. WebGL - und damit MediaPipe -
//! verweigert die Verarbeitung solcher Bilder, solange die Kamera-App keine
//! CORS-Header schickt. Das Backend holt die Einzelbilder deshalb selbst und
//! reicht sie als Rohdaten an das Overlay weiter; für den Webview stammen sie
//! damit aus der eigenen Anwendung.
//!
//! Der Abruf läuft ausschließlich gegen die vom Nutzer eingetragene Adresse und
//! nur, solange das Erkennungsfenster offen ist.

use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Verbindungsaufbau darf nicht hängen bleiben - im WLAN ist eine tote Adresse
/// der Normalfall (Handy aus, App nicht gestartet).
const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
/// Notbremse für stehende Verbindungen ohne Daten.
const BODY_TIMEOUT: Duration = Duration::from_secs(30);
/// Pause nach einem sauber beendeten Abruf (Adressen, die Einzelbilder liefern).
const RETRY_PAUSE: Duration = Duration::from_millis(150);
/// Nach einem Fehlversuch wird die Pause verdoppelt, bis zum Maximum.
///
/// Wichtig bei Kamera-Apps mit nur einem Stream-Platz: Wer im Sekundentakt neu
/// verbindet, hält den Platz dauerhaft besetzt und bekommt nur noch die
/// Bedienseite der App zurück. Langsameres Nachfragen löst genau das.
const ERROR_PAUSE_START: Duration = Duration::from_millis(700);
const ERROR_PAUSE_MAX: Duration = Duration::from_secs(3);
const READ_CHUNK: usize = 16 * 1024;
/// Über dieser Größe gilt der Puffer als verloren (kein JPEG in Sicht).
const MAX_BUFFER_BYTES: usize = 8 * 1024 * 1024;
/// Geduld beim einmaligen Verbindungstest.
const PROBE_TIMEOUT: Duration = Duration::from_secs(6);

/// Zuletzt empfangenes Einzelbild plus Verbindungsstatus.
///
/// Es gibt genau eine Verbindung zur Kamera, aber mehrere Abnehmer (Overlay und
/// Vorschau im Hauptfenster). Das ist keine Sparmaßnahme: verbreitete Kamera-Apps
/// bedienen nur einen Stream-Client und antworten dem zweiten mit ihrer
/// Bedienseite.
#[derive(Default)]
pub struct NetworkCamera {
    running: Arc<AtomicBool>,
    latest: Arc<Mutex<Option<Vec<u8>>>>,
    error: Arc<Mutex<Option<String>>>,
    /// Die Vorschau im Hauptfenster hält die Verbindung offen, auch wenn das
    /// Overlay sich wieder schließt.
    hold: Arc<AtomicBool>,
}

impl NetworkCamera {
    /// Startet den Abruf. Ein bereits laufender Abruf wird vorher beendet.
    pub fn start(&self, url: String) {
        self.stop();

        self.running.store(true, Ordering::SeqCst);
        *self.error.lock().expect("Kamera-Mutex") = None;
        let running = Arc::clone(&self.running);
        let latest = Arc::clone(&self.latest);
        let error = Arc::clone(&self.error);

        std::thread::spawn(move || {
            let mut error_pause = ERROR_PAUSE_START;
            while running.load(Ordering::SeqCst) {
                let pause = match pump(&url, &running, &latest) {
                    Ok(()) => {
                        *error.lock().expect("Kamera-Mutex") = None;
                        error_pause = ERROR_PAUSE_START;
                        RETRY_PAUSE
                    }
                    Err(message) => {
                        *error.lock().expect("Kamera-Mutex") = Some(message);
                        let pause = error_pause;
                        error_pause = (error_pause * 2).min(ERROR_PAUSE_MAX);
                        pause
                    }
                };
                if !running.load(Ordering::SeqCst) {
                    break;
                }
                std::thread::sleep(pause);
            }
        });
    }

    /// Startet nur, wenn noch nichts läuft - eine bestehende Verbindung wird
    /// nicht unnötig abgerissen.
    pub fn ensure_started(&self, url: String) {
        if self.running.load(Ordering::SeqCst) {
            return;
        }
        self.start(url);
    }

    /// Merkt vor, dass die Verbindung über das Overlay hinaus gebraucht wird.
    pub fn set_hold(&self, hold: bool) {
        self.hold.store(hold, Ordering::SeqCst);
    }

    pub fn is_held(&self) -> bool {
        self.hold.load(Ordering::SeqCst)
    }

    /// Beendet die Verbindung, sofern sie nicht von der Vorschau gehalten wird.
    pub fn stop_unless_held(&self) {
        if !self.is_held() {
            self.stop();
        }
    }

    /// Beendet den Abruf. Die letzte Fehlermeldung bleibt absichtlich stehen:
    /// Sie ist der einzige Hinweis darauf, warum das Overlay leer blieb.
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
        *self.latest.lock().expect("Kamera-Mutex") = None;
    }

    pub fn frame(&self) -> Option<Vec<u8>> {
        self.latest.lock().expect("Kamera-Mutex").clone()
    }

    pub fn error(&self) -> Option<String> {
        self.error.lock().expect("Kamera-Mutex").clone()
    }
}

/// Liest von der Adresse, bis der Stream endet oder der Abruf gestoppt wird.
///
/// Deckt beide gängigen Fälle ab: einen fortlaufenden MJPEG-Stream (`/video`)
/// und eine Adresse, die pro Aufruf ein Einzelbild liefert (`/shot.jpg`) - dort
/// endet der Stream nach einem Bild und die Schleife baut neu auf.
fn pump(url: &str, running: &AtomicBool, latest: &Mutex<Option<Vec<u8>>>) -> Result<(), String> {
    let mut response = connect(url)?;
    check_content_type(content_type(&response).as_deref())?;

    let mut reader = response.body_mut().as_reader();
    let mut buffer: Vec<u8> = Vec::with_capacity(READ_CHUNK * 4);
    let mut chunk = vec![0u8; READ_CHUNK];

    while running.load(Ordering::SeqCst) {
        let read = reader
            .read(&mut chunk)
            .map_err(|error| crate::i18n::msg_error("camera.aborted", error))?;
        if read == 0 {
            // Einzelbild-Adresse: Stream zu Ende, Rest des Puffers auswerten.
            if let Some(frame) = next_jpeg(&mut buffer) {
                *latest.lock().expect("Kamera-Mutex") = Some(frame);
            }
            return Ok(());
        }

        buffer.extend_from_slice(&chunk[..read]);
        while let Some(frame) = next_jpeg(&mut buffer) {
            *latest.lock().expect("Kamera-Mutex") = Some(frame);
        }
        if buffer.len() > MAX_BUFFER_BYTES {
            buffer.clear();
        }
    }
    Ok(())
}

fn connect(url: &str) -> Result<ureq::http::Response<ureq::Body>, String> {
    let agent = ureq::Agent::config_builder()
        .timeout_connect(Some(CONNECT_TIMEOUT))
        .timeout_recv_body(Some(BODY_TIMEOUT))
        .build()
        .new_agent();

    agent
        .get(url)
        .call()
        .map_err(|error| crate::i18n::msg_error("camera.unreachable", error))
}

fn content_type(response: &ureq::http::Response<ureq::Body>) -> Option<String> {
    response
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_ascii_lowercase())
}

/// Stellt sicher, dass wirklich Bilddaten kommen.
///
/// Wichtig in der Praxis: Manche Kamera-Apps (u. a. DroidCam) beantworten die
/// Stream-Adresse mit ihrer eigenen Bedienseite, sobald der Stream schon von
/// einem anderen Programm belegt ist. Ohne diese Prüfung würde die Erkennung
/// stumm ins Leere laufen - genau die schwarze Vorschau ohne Erklärung.
fn check_content_type(content_type: Option<&str>) -> Result<(), String> {
    match content_type {
        Some(value) if value.contains("multipart/") || value.starts_with("image/") => Ok(()),
        Some(value) if value.contains("text/html") => {
            Err(crate::i18n::msg("camera.html_response"))
        }
        Some(value) => Err(crate::i18n::msg_args(
            "camera.wrong_type",
            &[("type".into(), value.to_string())],
        )),
        None => Ok(()),
    }
}

/// Einmaliger Verbindungstest für die Einstellungen: verbinden, ein Bild holen,
/// Klartext zurückgeben.
pub fn probe(url: &str) -> Result<String, String> {
    let mut response = connect(url)?;
    let content_type = content_type(&response);
    check_content_type(content_type.as_deref())?;

    let mut reader = response.body_mut().as_reader();
    let mut buffer: Vec<u8> = Vec::with_capacity(READ_CHUNK * 4);
    let mut chunk = vec![0u8; READ_CHUNK];
    let deadline = Instant::now() + PROBE_TIMEOUT;

    while Instant::now() < deadline {
        let read = reader
            .read(&mut chunk)
            .map_err(|error| crate::i18n::msg_error("camera.probe_aborted", error))?;
        buffer.extend_from_slice(&chunk[..read]);

        if let Some(frame) = next_jpeg(&mut buffer) {
            let size = match jpeg_dimensions(&frame) {
                Some((width, height)) => crate::i18n::msg_args(
                    "camera.pixels",
                    &[
                        ("width".into(), width.to_string()),
                        ("height".into(), height.to_string()),
                    ],
                ),
                None => format!("{} KB", frame.len() / 1024),
            };
            let kind = crate::i18n::msg(
                if content_type.as_deref().is_some_and(|value| value.contains("multipart/")) {
                    "camera.kind_stream"
                } else {
                    "camera.kind_still"
                },
            );
            return Ok(crate::i18n::msg_args(
                "camera.probe_ok",
                &[("kind".into(), kind), ("size".into(), size)],
            ));
        }
        if read == 0 {
            break;
        }
    }
    Err(crate::i18n::msg("camera.no_frame"))
}

/// Liest Breite und Höhe aus dem SOF-Segment eines JPEG.
fn jpeg_dimensions(data: &[u8]) -> Option<(u16, u16)> {
    let mut index = 2;
    while index + 9 < data.len() {
        if data[index] != 0xFF {
            index += 1;
            continue;
        }
        let marker = data[index + 1];
        // SOF0-SOF15, ohne die Marker, die keine Bildmaße tragen.
        if (0xC0..=0xCF).contains(&marker) && !matches!(marker, 0xC4 | 0xC8 | 0xCC) {
            let height = u16::from_be_bytes([data[index + 5], data[index + 6]]);
            let width = u16::from_be_bytes([data[index + 7], data[index + 8]]);
            return Some((width, height));
        }
        let length = u16::from_be_bytes([data[index + 2], data[index + 3]]) as usize;
        if length < 2 {
            return None;
        }
        index += 2 + length;
    }
    None
}

/// Schneidet das nächste vollständige JPEG aus dem Puffer.
///
/// Gearbeitet wird über die JPEG-Marker statt über die Multipart-Grenzen: das
/// funktioniert bei allen verbreiteten Kamera-Apps gleich, unabhängig davon,
/// wie sauber sie ihre Multipart-Kopfzeilen setzen.
fn next_jpeg(buffer: &mut Vec<u8>) -> Option<Vec<u8>> {
    let start = find(buffer, &[0xFF, 0xD8, 0xFF], 0)?;
    let end = find(buffer, &[0xFF, 0xD9], start + 3)?;
    let frame = buffer[start..end + 2].to_vec();
    buffer.drain(..end + 2);
    Some(frame)
}

fn find(haystack: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    if from >= haystack.len() || needle.is_empty() {
        return None;
    }
    haystack[from..]
        .windows(needle.len())
        .position(|window| window == needle)
        .map(|index| index + from)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jpeg(payload: &[u8]) -> Vec<u8> {
        let mut data = vec![0xFF, 0xD8, 0xFF];
        data.extend_from_slice(payload);
        data.extend_from_slice(&[0xFF, 0xD9]);
        data
    }

    #[test]
    fn schneidet_einzelbilder_aus_dem_multipart_strom() {
        let mut buffer = Vec::new();
        buffer.extend_from_slice(b"--boundary\r\nContent-Type: image/jpeg\r\n\r\n");
        buffer.extend_from_slice(&jpeg(b"erstes"));
        buffer.extend_from_slice(b"\r\n--boundary\r\nContent-Type: image/jpeg\r\n\r\n");
        buffer.extend_from_slice(&jpeg(b"zweites"));

        let first = next_jpeg(&mut buffer).expect("erstes Bild");
        assert_eq!(first, jpeg(b"erstes"));
        let second = next_jpeg(&mut buffer).expect("zweites Bild");
        assert_eq!(second, jpeg(b"zweites"));
        assert!(next_jpeg(&mut buffer).is_none());
    }

    #[test]
    fn wartet_auf_das_vollstaendige_bild() {
        // Halbes Bild im Puffer: nichts ausliefern, sonst kommt Bruch bei
        // MediaPipe an.
        let mut buffer = vec![0xFF, 0xD8, 0xFF, 0x01, 0x02];
        assert!(next_jpeg(&mut buffer).is_none());
        assert_eq!(buffer.len(), 5, "unvollständige Daten bleiben erhalten");

        buffer.extend_from_slice(&[0xFF, 0xD9]);
        assert!(next_jpeg(&mut buffer).is_some());
    }

    #[test]
    fn ignoriert_vorspann_ohne_bilddaten() {
        let mut buffer = b"HTTP-Kopfzeilen ohne Bild".to_vec();
        assert!(next_jpeg(&mut buffer).is_none());
    }

    #[test]
    fn erkennt_eine_webseite_als_falsche_antwort() {
        crate::i18n::set_current(crate::i18n::Lang::De);
        let error = check_content_type(Some("text/html; charset=utf-8")).unwrap_err();
        assert!(error.contains("Webseite"), "{error}");
        assert!(error.contains("belegt"), "Hinweis auf belegten Stream fehlt: {error}");

        assert!(check_content_type(Some("multipart/x-mixed-replace;boundary=--dcmjpeg")).is_ok());
        assert!(check_content_type(Some("image/jpeg")).is_ok());
        assert!(check_content_type(None).is_ok(), "ohne Angabe einfach versuchen");
        assert!(check_content_type(Some("application/json")).is_err());
    }

    #[test]
    fn liest_bildmasse_aus_dem_jpeg() {
        // Minimales JPEG-Gerüst: SOI, APP0-Segment, SOF0 mit 640 x 480.
        let mut data = vec![0xFF, 0xD8];
        data.extend_from_slice(&[0xFF, 0xE0, 0x00, 0x04, 0x00, 0x00]);
        data.extend_from_slice(&[0xFF, 0xC0, 0x00, 0x11, 0x08]);
        data.extend_from_slice(&480u16.to_be_bytes());
        data.extend_from_slice(&640u16.to_be_bytes());
        data.extend_from_slice(&[0x03, 0x01, 0x22, 0x00, 0xFF, 0xD9]);

        assert_eq!(jpeg_dimensions(&data), Some((640, 480)));
        assert_eq!(jpeg_dimensions(&[0xFF, 0xD8, 0xFF, 0xD9]), None);
    }

    /// Kleiner MJPEG-Server auf einem freien Port - ersetzt im Test die
    /// Kamera-App auf dem Handy.
    fn serve_mjpeg(frames: usize) -> String {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("Testserver");
        let port = listener.local_addr().unwrap().port();

        std::thread::spawn(move || {
            for stream in listener.incoming().take(1) {
                let Ok(mut stream) = stream else { continue };
                use std::io::Write;

                let mut body = Vec::new();
                for index in 0..frames {
                    body.extend_from_slice(
                        b"--frameboundary\r\nContent-Type: image/jpeg\r\n\r\n",
                    );
                    body.extend_from_slice(&jpeg(format!("bild-{index}").as_bytes()));
                    body.extend_from_slice(b"\r\n");
                }

                let header = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: multipart/x-mixed-replace; \
                     boundary=frameboundary\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = stream.write_all(header.as_bytes());
                let _ = stream.write_all(&body);
                let _ = stream.flush();
            }
        });

        format!("http://127.0.0.1:{port}/video")
    }

    #[test]
    fn holt_einzelbilder_von_einer_netzwerk_kamera() {
        let camera = NetworkCamera::default();
        camera.start(serve_mjpeg(2));

        let mut frame = None;
        for _ in 0..100 {
            if let Some(data) = camera.frame() {
                frame = Some(data);
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        camera.stop();

        let frame = frame.expect("kein Bild von der Testkamera erhalten");
        assert!(camera.error().is_none(), "erfolgreicher Abruf darf keinen Fehler melden");
        assert_eq!(&frame[..3], &[0xFF, 0xD8, 0xFF], "kein JPEG-Anfang");
        assert_eq!(&frame[frame.len() - 2..], &[0xFF, 0xD9], "kein JPEG-Ende");
        assert!(camera.frame().is_none(), "stop() muss das Bild verwerfen");
    }

    /// Prüft eine echte Kamera im Netz - läuft nur auf Zuruf, weil dafür ein
    /// Gerät erreichbar sein muss:
    ///
    /// ```text
    /// KAMERA_URL=http://192.168.1.20:4747/video \
    ///   cargo test --lib -- --ignored --nocapture echte_kamera
    /// ```
    #[test]
    #[ignore = "braucht eine erreichbare Kamera im Netz"]
    fn echte_kamera() {
        let url = std::env::var("KAMERA_URL").expect("KAMERA_URL setzen");
        match probe(&url) {
            Ok(info) => println!("OK: {info}"),
            Err(error) => panic!("{error}"),
        }
    }

    #[test]
    fn meldet_eine_tote_adresse_als_fehler() {
        let camera = NetworkCamera::default();
        // Port ohne Server: die Verbindung schlägt sofort fehl.
        camera.start("http://127.0.0.1:1/video".to_string());

        let mut error = None;
        for _ in 0..100 {
            if let Some(message) = camera.error() {
                error = Some(message);
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        camera.stop();

        let error = error.expect("kein Fehler gemeldet");
        assert!(
            error.contains("nicht erreichbar") || error.contains("abgebrochen"),
            "unerwartete Meldung: {error}"
        );
    }
}
