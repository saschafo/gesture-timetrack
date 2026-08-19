//! SQLite-Anbindung. Die Datenbank liegt im App-Data-Verzeichnis des Nutzers
//! und verlässt das Gerät nie - es gibt keinerlei Sync- oder Upload-Pfad.

use std::path::Path;
use std::sync::Mutex;

use chrono::{DateTime, Local, NaiveDateTime, TimeZone};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

/// Zeitstempelformat in der Datenbank: lokale Zeit, damit `date(start_ts)`
/// direkt in SQL für Tagesauswertungen funktioniert.
pub const TS_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

pub fn format_ts(ts: DateTime<Local>) -> String {
    ts.format(TS_FORMAT).to_string()
}

pub fn parse_ts(raw: &str) -> AppResult<DateTime<Local>> {
    let naive = NaiveDateTime::parse_from_str(raw, TS_FORMAT)
        .map_err(|_| AppError::args("db.bad_timestamp", &[("value", &raw)]))?;
    Local
        .from_local_datetime(&naive)
        .single()
        .ok_or_else(|| AppError::args("entry.ambiguous_timestamp", &[("value", &raw)]))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: i64,
    pub name: String,
    pub color: String,
    pub active: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TimeEntry {
    pub id: i64,
    pub project_id: Option<i64>,
    pub project_name: String,
    pub start_ts: String,
    pub end_ts: Option<String>,
    pub pause_duration_seconds: i64,
    pub duration_seconds: i64,
    pub gesture_triggered: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct CustomGesture {
    pub id: i64,
    pub name: String,
    /// Auszulösende Aktion, siehe state::CustomAction.
    pub action: String,
    /// Nur bei der Aktion „Projekt starten" gesetzt.
    pub project_id: Option<i64>,
    pub project_name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GestureSample {
    pub gesture: String,
    pub features: Vec<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectTotal {
    pub project_id: Option<i64>,
    pub project_name: String,
    pub color: String,
    pub seconds: i64,
}

pub struct Db(pub Mutex<Connection>);

impl Db {
    pub fn open(path: &Path) -> AppResult<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let db = Db(Mutex::new(conn));
        db.migrate()?;
        Ok(db)
    }

    pub fn lock(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.0.lock().expect("DB-Mutex vergiftet")
    }

    fn migrate(&self) -> AppResult<()> {
        let conn = self.lock();
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS projects (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              name TEXT NOT NULL,
              color TEXT,
              active BOOLEAN DEFAULT 1,
              created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS time_entries (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              project_id INTEGER REFERENCES projects(id),
              start_ts DATETIME NOT NULL,
              end_ts DATETIME,
              pause_duration_seconds INTEGER DEFAULT 0,
              duration_seconds INTEGER,
              gesture_triggered BOOLEAN DEFAULT 1,
              created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS settings (
              key TEXT PRIMARY KEY,
              value TEXT
            );

            -- Eingelernte Gesten: pro Aufnahme ein Merkmalsvektor als JSON.
            -- `version` gehört zum Merkmalssatz aus src/gesture/features.ts -
            -- ändert der sich, werden alte Aufnahmen einfach ignoriert statt
            -- falsch interpretiert.
            CREATE TABLE IF NOT EXISTS gesture_samples (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              gesture TEXT NOT NULL,
              version INTEGER NOT NULL DEFAULT 1,
              features TEXT NOT NULL,
              created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

            -- Selbst definierte Gesten. Sie funktionieren ausschließlich über
            -- eingelernte Aufnahmen; ihre Kennung steht in gesture_samples als
            -- "custom:<id>".
            CREATE TABLE IF NOT EXISTS custom_gestures (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              name TEXT NOT NULL,
              action TEXT NOT NULL,
              project_id INTEGER REFERENCES projects(id),
              created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

            CREATE INDEX IF NOT EXISTS idx_entries_start ON time_entries(start_ts);
            CREATE INDEX IF NOT EXISTS idx_entries_project ON time_entries(project_id);
            CREATE INDEX IF NOT EXISTS idx_samples_gesture ON gesture_samples(gesture, version);
            "#,
        )?;
        Ok(())
    }

    // ---------- Projekte ----------

    pub fn projects(&self, only_active: bool) -> AppResult<Vec<Project>> {
        let conn = self.lock();
        let sql = if only_active {
            "SELECT id, name, COALESCE(color, '#4f46e5'), active, created_at
               FROM projects WHERE active = 1 ORDER BY name COLLATE NOCASE"
        } else {
            "SELECT id, name, COALESCE(color, '#4f46e5'), active, created_at
               FROM projects ORDER BY active DESC, name COLLATE NOCASE"
        };
        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map([], |row| {
            Ok(Project {
                id: row.get(0)?,
                name: row.get(1)?,
                color: row.get(2)?,
                active: row.get(3)?,
                created_at: row.get(4)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn project(&self, id: i64) -> AppResult<Option<Project>> {
        let conn = self.lock();
        let project = conn
            .query_row(
                "SELECT id, name, COALESCE(color, '#4f46e5'), active, created_at
                   FROM projects WHERE id = ?1",
                params![id],
                |row| {
                    Ok(Project {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        color: row.get(2)?,
                        active: row.get(3)?,
                        created_at: row.get(4)?,
                    })
                },
            )
            .optional()?;
        Ok(project)
    }

    pub fn create_project(&self, name: &str, color: &str) -> AppResult<i64> {
        let name = name.trim();
        if name.is_empty() {
            return Err(AppError::key("project.name_empty"));
        }
        let conn = self.lock();
        conn.execute(
            "INSERT INTO projects (name, color, active) VALUES (?1, ?2, 1)",
            params![name, color],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn update_project(&self, id: i64, name: &str, color: &str, active: bool) -> AppResult<()> {
        let name = name.trim();
        if name.is_empty() {
            return Err(AppError::key("project.name_empty"));
        }
        let conn = self.lock();
        conn.execute(
            "UPDATE projects SET name = ?2, color = ?3, active = ?4 WHERE id = ?1",
            params![id, name, color, active],
        )?;
        Ok(())
    }

    /// Projekte werden nur dann echt gelöscht, wenn keine Zeiteinträge daran
    /// hängen - sonst würde Abrechnungshistorie verschwinden.
    pub fn delete_project(&self, id: i64) -> AppResult<bool> {
        let conn = self.lock();
        let used: i64 = conn.query_row(
            "SELECT COUNT(*) FROM time_entries WHERE project_id = ?1",
            params![id],
            |row| row.get(0),
        )?;
        if used > 0 {
            conn.execute("UPDATE projects SET active = 0 WHERE id = ?1", params![id])?;
            return Ok(false);
        }
        conn.execute("DELETE FROM projects WHERE id = ?1", params![id])?;
        Ok(true)
    }

    // ---------- Zeiteinträge ----------

    pub fn open_entry(
        &self,
        project_id: i64,
        start: DateTime<Local>,
        gesture_triggered: bool,
    ) -> AppResult<i64> {
        let conn = self.lock();
        conn.execute(
            "INSERT INTO time_entries (project_id, start_ts, pause_duration_seconds, gesture_triggered)
             VALUES (?1, ?2, 0, ?3)",
            params![project_id, format_ts(start), gesture_triggered],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn close_entry(
        &self,
        entry_id: i64,
        end: DateTime<Local>,
        pause_seconds: i64,
        duration_seconds: i64,
    ) -> AppResult<()> {
        let conn = self.lock();
        conn.execute(
            "UPDATE time_entries
                SET end_ts = ?2, pause_duration_seconds = ?3, duration_seconds = ?4
              WHERE id = ?1",
            params![entry_id, format_ts(end), pause_seconds, duration_seconds],
        )?;
        Ok(())
    }

    /// Offener Eintrag aus einer früheren Sitzung (z. B. nach einem Absturz).
    pub fn dangling_entry(&self) -> AppResult<Option<(i64, i64, DateTime<Local>, i64)>> {
        let conn = self.lock();
        let row = conn
            .query_row(
                "SELECT id, project_id, start_ts, COALESCE(pause_duration_seconds, 0)
                   FROM time_entries
                  WHERE end_ts IS NULL AND project_id IS NOT NULL
                  ORDER BY id DESC LIMIT 1",
                [],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, i64>(3)?,
                    ))
                },
            )
            .optional()?;
        drop(conn);
        match row {
            Some((id, project_id, start_raw, pause)) => {
                Ok(Some((id, project_id, parse_ts(&start_raw)?, pause)))
            }
            None => Ok(None),
        }
    }

    /// Einzelner Eintrag - gebraucht für die Prüfungen beim Bearbeiten.
    pub fn entry(&self, id: i64) -> AppResult<Option<TimeEntry>> {
        let conn = self.lock();
        let entry = conn
            .query_row(
                "SELECT e.id, e.project_id, COALESCE(p.name, ''), e.start_ts, e.end_ts,
                        COALESCE(e.pause_duration_seconds, 0), COALESCE(e.duration_seconds, 0),
                        COALESCE(e.gesture_triggered, 0)
                   FROM time_entries e
                   LEFT JOIN projects p ON p.id = e.project_id
                  WHERE e.id = ?1",
                params![id],
                |row| {
                    Ok(TimeEntry {
                        id: row.get(0)?,
                        project_id: row.get(1)?,
                        project_name: row.get(2)?,
                        start_ts: row.get(3)?,
                        end_ts: row.get(4)?,
                        pause_duration_seconds: row.get(5)?,
                        duration_seconds: row.get(6)?,
                        gesture_triggered: row.get(7)?,
                    })
                },
            )
            .optional()?;
        Ok(entry)
    }

    /// Vollständiger Eintrag, von Hand erfasst.
    pub fn insert_entry(
        &self,
        project_id: i64,
        start: DateTime<Local>,
        end: DateTime<Local>,
        pause_seconds: i64,
        duration_seconds: i64,
    ) -> AppResult<i64> {
        let conn = self.lock();
        conn.execute(
            "INSERT INTO time_entries
               (project_id, start_ts, end_ts, pause_duration_seconds, duration_seconds,
                gesture_triggered)
             VALUES (?1, ?2, ?3, ?4, ?5, 0)",
            params![
                project_id,
                format_ts(start),
                format_ts(end),
                pause_seconds,
                duration_seconds
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn update_entry(
        &self,
        id: i64,
        project_id: i64,
        start: DateTime<Local>,
        end: DateTime<Local>,
        pause_seconds: i64,
        duration_seconds: i64,
    ) -> AppResult<()> {
        let conn = self.lock();
        conn.execute(
            "UPDATE time_entries
                SET project_id = ?2, start_ts = ?3, end_ts = ?4,
                    pause_duration_seconds = ?5, duration_seconds = ?6
              WHERE id = ?1",
            params![
                id,
                project_id,
                format_ts(start),
                format_ts(end),
                pause_seconds,
                duration_seconds
            ],
        )?;
        Ok(())
    }

    pub fn delete_entry(&self, id: i64) -> AppResult<()> {
        let conn = self.lock();
        conn.execute("DELETE FROM time_entries WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// Einträge eines Zeitraums, optional auf ein Projekt begrenzt.
    pub fn entries_between(
        &self,
        from: &str,
        to: &str,
        project_id: Option<i64>,
    ) -> AppResult<Vec<TimeEntry>> {
        let conn = self.lock();
        // Ein einziges Statement für beide Fälle: `?3 IS NULL` heißt „alle
        // Projekte" - das erspart zwei fast gleiche Abfragen.
        let mut stmt = conn.prepare(
            "SELECT e.id, e.project_id, COALESCE(p.name, ''), e.start_ts, e.end_ts,
                    COALESCE(e.pause_duration_seconds, 0), COALESCE(e.duration_seconds, 0),
                    COALESCE(e.gesture_triggered, 0)
               FROM time_entries e
               LEFT JOIN projects p ON p.id = e.project_id
              WHERE date(e.start_ts) BETWEEN date(?1) AND date(?2)
                AND (?3 IS NULL OR e.project_id = ?3)
              ORDER BY e.start_ts DESC",
        )?;
        let rows = stmt.query_map(params![from, to, project_id], |row| {
            Ok(TimeEntry {
                id: row.get(0)?,
                project_id: row.get(1)?,
                project_name: row.get(2)?,
                start_ts: row.get(3)?,
                end_ts: row.get(4)?,
                pause_duration_seconds: row.get(5)?,
                duration_seconds: row.get(6)?,
                gesture_triggered: row.get(7)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Summen je Projekt für einen Tag - Grundlage der Tray-Übersicht.
    pub fn totals_for_day(&self, day: &str) -> AppResult<Vec<ProjectTotal>> {
        let conn = self.lock();
        let mut stmt = conn.prepare(
            "SELECT e.project_id, COALESCE(p.name, ''), COALESCE(p.color, '#4f46e5'),
                    SUM(COALESCE(e.duration_seconds, 0))
               FROM time_entries e
               LEFT JOIN projects p ON p.id = e.project_id
              WHERE date(e.start_ts) = date(?1) AND e.end_ts IS NOT NULL
              GROUP BY e.project_id
              ORDER BY 4 DESC",
        )?;
        let rows = stmt.query_map(params![day], |row| {
            Ok(ProjectTotal {
                project_id: row.get(0)?,
                project_name: row.get(1)?,
                color: row.get(2)?,
                seconds: row.get(3)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn day_total(&self, day: &str) -> AppResult<i64> {
        let conn = self.lock();
        let seconds: i64 = conn.query_row(
            "SELECT COALESCE(SUM(duration_seconds), 0)
               FROM time_entries
              WHERE date(start_ts) = date(?1) AND end_ts IS NOT NULL",
            params![day],
            |row| row.get(0),
        )?;
        Ok(seconds)
    }

    // ---------- Eigene Gesten ----------

    pub fn custom_gestures(&self) -> AppResult<Vec<CustomGesture>> {
        let conn = self.lock();
        let mut stmt = conn.prepare(
            "SELECT g.id, g.name, g.action, g.project_id, p.name
               FROM custom_gestures g
               LEFT JOIN projects p ON p.id = g.project_id
              ORDER BY g.id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(CustomGesture {
                id: row.get(0)?,
                name: row.get(1)?,
                action: row.get(2)?,
                project_id: row.get(3)?,
                project_name: row.get(4)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn custom_gesture(&self, id: i64) -> AppResult<Option<CustomGesture>> {
        Ok(self.custom_gestures()?.into_iter().find(|item| item.id == id))
    }

    pub fn create_custom_gesture(
        &self,
        name: &str,
        action: &str,
        project_id: Option<i64>,
    ) -> AppResult<i64> {
        let name = name.trim();
        if name.is_empty() {
            return Err(AppError::key("gesture.custom_needs_name"));
        }
        let conn = self.lock();
        conn.execute(
            "INSERT INTO custom_gestures (name, action, project_id) VALUES (?1, ?2, ?3)",
            params![name, action, project_id],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn update_custom_gesture(
        &self,
        id: i64,
        name: &str,
        action: &str,
        project_id: Option<i64>,
    ) -> AppResult<()> {
        let name = name.trim();
        if name.is_empty() {
            return Err(AppError::key("gesture.custom_needs_name"));
        }
        let conn = self.lock();
        conn.execute(
            "UPDATE custom_gestures SET name = ?2, action = ?3, project_id = ?4 WHERE id = ?1",
            params![id, name, action, project_id],
        )?;
        Ok(())
    }

    /// Löscht die Geste samt ihrer Aufnahmen - eine Geste ohne Aufnahmen wäre
    /// wirkungslos, und Aufnahmen ohne Geste sind Datenmüll.
    pub fn delete_custom_gesture(&self, id: i64) -> AppResult<()> {
        let mut conn = self.lock();
        let transaction = conn.transaction()?;
        transaction.execute(
            "DELETE FROM gesture_samples WHERE gesture = ?1",
            params![format!("custom:{id}")],
        )?;
        transaction.execute("DELETE FROM custom_gestures WHERE id = ?1", params![id])?;
        transaction.commit()?;
        Ok(())
    }

    // ---------- Eingelernte Gesten ----------

    /// Speichert mehrere Aufnahmen einer Geste in einem Rutsch.
    pub fn add_gesture_samples(
        &self,
        gesture: &str,
        version: i64,
        samples: &[Vec<f64>],
    ) -> AppResult<usize> {
        let mut conn = self.lock();
        let transaction = conn.transaction()?;
        {
            let mut stmt = transaction.prepare(
                "INSERT INTO gesture_samples (gesture, version, features) VALUES (?1, ?2, ?3)",
            )?;
            for sample in samples {
                let json = serde_json::to_string(sample)
                    .map_err(|error| AppError::args("db.features_unreadable", &[("error", &error)]))?;
                stmt.execute(params![gesture, version, json])?;
            }
        }
        transaction.commit()?;
        Ok(samples.len())
    }

    pub fn gesture_samples(&self, version: i64) -> AppResult<Vec<GestureSample>> {
        let conn = self.lock();
        let mut stmt = conn.prepare(
            "SELECT gesture, features FROM gesture_samples WHERE version = ?1 ORDER BY id",
        )?;
        let rows = stmt.query_map(params![version], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        let mut samples = Vec::new();
        for row in rows {
            let (gesture, json) = row?;
            // Kaputte Zeilen überspringen statt die Erkennung lahmzulegen.
            if let Ok(features) = serde_json::from_str::<Vec<f64>>(&json) {
                samples.push(GestureSample { gesture, features });
            }
        }
        Ok(samples)
    }

    pub fn gesture_sample_counts(&self, version: i64) -> AppResult<Vec<(String, i64)>> {
        let conn = self.lock();
        let mut stmt = conn.prepare(
            "SELECT gesture, COUNT(*) FROM gesture_samples
              WHERE version = ?1 GROUP BY gesture ORDER BY gesture",
        )?;
        let rows = stmt.query_map(params![version], |row| Ok((row.get(0)?, row.get(1)?)))?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Entfernt Aufnahmen zu Gesten, die es nicht mehr gibt.
    ///
    /// Nötig, wenn das Vokabular sich ändert - etwa als „Daumen runter" durch
    /// „Daumen seitlich" ersetzt wurde. Ohne das Aufräumen läge in der Datenbank
    /// eine Kennung, zu der keine Geste mehr gehört; die Erkennung würde sie
    /// vorschlagen und das Backend sie nicht annehmen.
    pub fn prune_gesture_samples(&self, known: &[&str]) -> AppResult<usize> {
        let conn = self.lock();
        let placeholders = vec!["?"; known.len()].join(", ");
        let sql = format!(
            "DELETE FROM gesture_samples
              WHERE gesture NOT IN ({placeholders})
                AND gesture NOT IN (SELECT 'custom:' || id FROM custom_gestures)"
        );
        let removed = conn.execute(&sql, rusqlite::params_from_iter(known.iter()))?;
        Ok(removed)
    }

    /// Löscht die Aufnahmen einer Geste - oder alle, wenn keine genannt ist.
    pub fn clear_gesture_samples(&self, gesture: Option<&str>) -> AppResult<usize> {
        let conn = self.lock();
        let removed = match gesture {
            Some(gesture) => {
                conn.execute("DELETE FROM gesture_samples WHERE gesture = ?1", params![gesture])?
            }
            None => conn.execute("DELETE FROM gesture_samples", [])?,
        };
        Ok(removed)
    }

    // ---------- Einstellungen ----------

    pub fn setting(&self, key: &str) -> AppResult<Option<String>> {
        let conn = self.lock();
        let value = conn
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                params![key],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        Ok(value)
    }

    pub fn set_setting(&self, key: &str, value: &str) -> AppResult<()> {
        let conn = self.lock();
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }
}

/// Wie lange ein offener Eintrag nach einem Neustart noch als "läuft weiter"
/// gilt. Danach wird er verworfen, statt eine Nachtschicht zu erfinden.
pub const RESUME_LIMIT_SECONDS: i64 = 12 * 60 * 60;

#[cfg(test)]
pub(crate) mod testing {
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::Db;

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    /// Datenbank in einer temporären Datei - `rusqlite`-Speicherdatenbanken
    /// würden pro Verbindung neu anfangen, hier soll echtes SQL laufen.
    pub struct TempDb {
        pub db: Db,
        path: PathBuf,
    }

    impl TempDb {
        pub fn new() -> Self {
            let id = COUNTER.fetch_add(1, Ordering::SeqCst);
            let path = std::env::temp_dir().join(format!(
                "gesture-timetrack-test-{}-{id}.sqlite3",
                std::process::id()
            ));
            let _ = std::fs::remove_file(&path);
            let db = Db::open(&path).expect("Testdatenbank");
            TempDb { db, path }
        }
    }

    impl Drop for TempDb {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.path);
            let _ = std::fs::remove_file(self.path.with_extension("sqlite3-wal"));
            let _ = std::fs::remove_file(self.path.with_extension("sqlite3-shm"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::testing::TempDb;
    use super::*;
    use chrono::Duration;

    #[test]
    fn projekte_anlegen_lesen_und_deaktivieren() {
        let temp = TempDb::new();
        let db = &temp.db;

        let id = db.create_project("Kunde Meier", "#4f46e5").unwrap();
        assert_eq!(db.projects(true).unwrap().len(), 1);

        db.update_project(id, "Kunde Meier – Website", "#16a34a", false)
            .unwrap();
        assert!(db.projects(true).unwrap().is_empty());
        assert_eq!(db.projects(false).unwrap().len(), 1);

        let project = db.project(id).unwrap().unwrap();
        assert_eq!(project.name, "Kunde Meier – Website");
        assert!(!project.active);
    }

    #[test]
    fn leerer_projektname_wird_abgelehnt() {
        let temp = TempDb::new();
        assert!(temp.db.create_project("   ", "#000000").is_err());
    }

    #[test]
    fn projekt_mit_zeiten_wird_nur_deaktiviert() {
        let temp = TempDb::new();
        let db = &temp.db;

        let leer = db.create_project("Ohne Zeiten", "#000000").unwrap();
        let genutzt = db.create_project("Mit Zeiten", "#000000").unwrap();
        let entry = db.open_entry(genutzt, Local::now(), true).unwrap();
        db.close_entry(entry, Local::now(), 0, 60).unwrap();

        assert!(db.delete_project(leer).unwrap(), "leeres Projekt löschbar");
        assert!(
            !db.delete_project(genutzt).unwrap(),
            "Projekt mit Zeiten darf nicht verschwinden"
        );
        assert!(db.project(genutzt).unwrap().is_some());
        assert!(!db.project(genutzt).unwrap().unwrap().active);
    }

    #[test]
    fn tagessumme_zaehlt_nur_abgeschlossene_eintraege() {
        let temp = TempDb::new();
        let db = &temp.db;
        let now = Local::now();
        let today = now.format("%Y-%m-%d").to_string();

        let project = db.create_project("Projekt", "#4f46e5").unwrap();
        let closed = db.open_entry(project, now - Duration::hours(2), true).unwrap();
        db.close_entry(closed, now - Duration::hours(1), 300, 3300)
            .unwrap();
        // Noch laufender Eintrag: darf die Tagessumme nicht verfälschen.
        db.open_entry(project, now, true).unwrap();

        assert_eq!(db.day_total(&today).unwrap(), 3300);

        let totals = db.totals_for_day(&today).unwrap();
        assert_eq!(totals.len(), 1);
        assert_eq!(totals[0].seconds, 3300);
        assert_eq!(totals[0].project_name, "Projekt");
    }

    #[test]
    fn eintraege_lassen_sich_anlegen_aendern_und_loeschen() {
        let temp = TempDb::new();
        let db = &temp.db;
        let now = Local::now();
        let today = now.format("%Y-%m-%d").to_string();

        let alt = db.create_project("Alt", "#000000").unwrap();
        let neu = db.create_project("Neu", "#111111").unwrap();

        // Von Hand angelegter Eintrag: zwei Stunden, davon 30 Minuten Pause.
        let id = db
            .insert_entry(alt, now - Duration::hours(2), now, 1800, 5400)
            .unwrap();
        let entry = db.entry(id).unwrap().unwrap();
        assert_eq!(entry.duration_seconds, 5400);
        assert!(!entry.gesture_triggered, "manuell erfasst");
        assert_eq!(db.day_total(&today).unwrap(), 5400);

        // Umbuchen auf ein anderes Projekt, kürzere Zeit.
        db.update_entry(id, neu, now - Duration::hours(1), now, 0, 3600)
            .unwrap();
        let entry = db.entry(id).unwrap().unwrap();
        assert_eq!(entry.project_id, Some(neu));
        assert_eq!(entry.project_name, "Neu");
        assert_eq!(entry.pause_duration_seconds, 0);
        assert_eq!(db.day_total(&today).unwrap(), 3600);

        db.delete_entry(id).unwrap();
        assert!(db.entry(id).unwrap().is_none());
        assert_eq!(db.day_total(&today).unwrap(), 0);
    }

    #[test]
    fn eintraege_werden_nach_datum_gefiltert() {
        let temp = TempDb::new();
        let db = &temp.db;
        let now = Local::now();

        let project = db.create_project("Projekt", "#4f46e5").unwrap();
        let old = db.open_entry(project, now - Duration::days(10), false).unwrap();
        db.close_entry(old, now - Duration::days(10), 0, 600).unwrap();
        let recent = db.open_entry(project, now, true).unwrap();
        db.close_entry(recent, now, 0, 900).unwrap();

        let today = now.format("%Y-%m-%d").to_string();
        let entries = db.entries_between(&today, &today, None).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].duration_seconds, 900);
        assert!(entries[0].gesture_triggered);

        let von = (now - Duration::days(30)).format("%Y-%m-%d").to_string();
        assert_eq!(db.entries_between(&von, &today, None).unwrap().len(), 2);
    }

    #[test]
    fn eintraege_lassen_sich_nach_projekt_filtern() {
        let temp = TempDb::new();
        let db = &temp.db;
        let now = Local::now();
        let today = now.format("%Y-%m-%d").to_string();

        let a = db.create_project("A", "#000000").unwrap();
        let b = db.create_project("B", "#111111").unwrap();
        for (project, seconds) in [(a, 600), (a, 300), (b, 900)] {
            let id = db.open_entry(project, now, true).unwrap();
            db.close_entry(id, now, 0, seconds).unwrap();
        }

        assert_eq!(db.entries_between(&today, &today, None).unwrap().len(), 3);
        let only_a = db.entries_between(&today, &today, Some(a)).unwrap();
        assert_eq!(only_a.len(), 2);
        assert!(only_a.iter().all(|entry| entry.project_name == "A"));
        assert_eq!(db.entries_between(&today, &today, Some(b)).unwrap().len(), 1);
    }

    #[test]
    fn offener_eintrag_wird_gefunden() {
        let temp = TempDb::new();
        let db = &temp.db;
        let start = Local::now() - Duration::minutes(90);

        let project = db.create_project("Projekt", "#4f46e5").unwrap();
        assert!(db.dangling_entry().unwrap().is_none());

        let entry = db.open_entry(project, start, true).unwrap();
        let (id, found_project, found_start, pause) = db.dangling_entry().unwrap().unwrap();
        assert_eq!((id, found_project, pause), (entry, project, 0));
        // Sekundengenau, da die Datenbank ohne Bruchteile speichert.
        assert!((found_start - start).num_seconds().abs() <= 1);

        db.close_entry(entry, Local::now(), 0, 5400).unwrap();
        assert!(db.dangling_entry().unwrap().is_none());
    }

    #[test]
    fn veraltete_gestenaufnahmen_werden_entfernt() {
        let temp = TempDb::new();
        let db = &temp.db;
        let features = vec![vec![0.5; 10]];

        let eigene = db.create_custom_gesture("Handkante", "stop", None).unwrap();
        db.add_gesture_samples("open_hand", 1, &features).unwrap();
        db.add_gesture_samples("thumb_down", 1, &features).unwrap();
        db.add_gesture_samples(&format!("custom:{eigene}"), 1, &features)
            .unwrap();

        // „thumb_down" gehört nicht mehr zum Vokabular und muss verschwinden;
        // eigene Gesten bleiben.
        let removed = db
            .prune_gesture_samples(&["open_hand", "fist", "thumb_up", "thumb_side"])
            .unwrap();
        assert_eq!(removed, 1);

        let labels: Vec<String> = db
            .gesture_samples(1)
            .unwrap()
            .into_iter()
            .map(|sample| sample.gesture)
            .collect();
        assert!(labels.contains(&"open_hand".to_string()));
        assert!(labels.contains(&format!("custom:{eigene}")));
        assert!(!labels.iter().any(|label| label == "thumb_down"));
    }

    #[test]
    fn einstellungen_werden_ueberschrieben() {
        let temp = TempDb::new();
        let db = &temp.db;

        assert!(db.setting("hotkey").unwrap().is_none());
        db.set_setting("hotkey", "CommandOrControl+Alt+Space").unwrap();
        db.set_setting("hotkey", "CommandOrControl+Shift+T").unwrap();
        assert_eq!(
            db.setting("hotkey").unwrap().as_deref(),
            Some("CommandOrControl+Shift+T")
        );
    }
}
