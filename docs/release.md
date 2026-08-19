# Bauen und Ausliefern

Aus derselben Codebasis entstehen native Programme für **Windows 11** und
**macOS**. Tauri baut jeweils auf dem Zielsystem – ein macOS-Rechner kann kein
Windows-Paket erzeugen und umgekehrt.

## Was dabei herauskommt

| System | Ergebnis | Bemerkung |
|---|---|---|
| macOS | `.app` im `.dmg` | Universal möglich (Intel + Apple Silicon) |
| Windows 11 | `.msi` und `.exe` (NSIS) | WebView2 ist auf Windows 11 vorinstalliert |

```bash
npm install          # holt auch Modell und WASM-Laufzeit
npm run tauri:build
```

Die Pakete liegen danach unter `src-tauri/target/release/bundle/`.

Für ein Universal-Paket auf macOS:

```bash
rustup target add aarch64-apple-darwin x86_64-apple-darwin
npm run tauri:build -- --target universal-apple-darwin
```

## Version

Die Versionsnummer steht an drei Stellen (`package.json`,
`src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`). Ein Skript hält sie
zusammen:

```bash
npm run version:set 0.2.0
```

Laufen die Nummern auseinander, trägt der Installer eine andere Version als die
Anwendung meldet – und eine spätere Update-Prüfung erkennt womöglich gar nicht,
dass ein Update vorliegt. Die Anwendung zeigt ihre Version im Fensterkopf; sie
liest sie über `getVersion()` aus sich selbst, nicht aus einer zweiten Quelle.

## Signieren

Ohne Signatur läuft die App, aber die Systeme warnen deutlich.

* **macOS**: Apple Developer ID (99 $/Jahr) für Signatur und Notarisierung.
  Ohne Notarisierung meldet Gatekeeper beim ersten Start, die App sei nicht
  überprüft. Nötige Umgebungsvariablen: `APPLE_CERTIFICATE`,
  `APPLE_CERTIFICATE_PASSWORD`, `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID`.
* **Windows**: Code-Signing-Zertifikat (OV oder EV). Ohne Signatur zeigt
  SmartScreen eine Warnung, die erst mit wachsender Verbreitung verschwindet.
* Für die Kamera braucht macOS zusätzlich den Hinweistext aus
  `src-tauri/Info.plist` – der ist bereits enthalten.

## Apple-Signatur nachrüsten

Der Workflow baut **ohne** Apple-Signatur. Das ist Absicht: Reicht man die
Variablen leer durch, versucht Tauri ein leeres Zertifikat zu importieren und
der Bau bricht ab (`failed to import keychain certificate`). Erst wenn eine
Developer ID vorliegt, gehört dieser Block in `release.yml` unter `env:`:

```yaml
          APPLE_CERTIFICATE: ${{ secrets.APPLE_CERTIFICATE }}
          APPLE_CERTIFICATE_PASSWORD: ${{ secrets.APPLE_CERTIFICATE_PASSWORD }}
          APPLE_SIGNING_IDENTITY: ${{ secrets.APPLE_SIGNING_IDENTITY }}
          APPLE_ID: ${{ secrets.APPLE_ID }}
          APPLE_PASSWORD: ${{ secrets.APPLE_PASSWORD }}
          APPLE_TEAM_ID: ${{ secrets.APPLE_TEAM_ID }}
```

Alle sechs Secrets müssen dann tatsächlich gefüllt sein - halbe Sachen führen zu
demselben Abbruch.

## Auf beiden Systemen prüfen

Der Kern ist plattformunabhängig, drei Stellen verhalten sich aber
unterschiedlich und gehören auf Windows getestet:

1. **Kamerafreigabe**: macOS fragt einmalig über die Systemabfrage. Unter
   Windows fragt WebView2 – das ist der Punkt, der am ehesten Nacharbeit
   braucht.
2. **Menüleiste vs. Taskleiste**: Die Uhrzeit neben dem Symbol gibt es nur unter
   macOS; unter Windows zeigt der Tooltip den Stand. Das kleine Fenster öffnet
   sich unter Windows über der Taskleiste statt unter der Menüleiste
   (`src-tauri/src/panel.rs`).
3. **Hotkey**: `CommandOrControl` wird zu `Strg`. Belegte Kombinationen
   unterscheiden sich je System – die Anwendung meldet das im Klartext.

## Updates über GitHub Releases

Die Anwendung prüft **nur auf Knopfdruck** (*Einstellungen → Aktualisierung →
Nach Updates suchen*), niemals im Hintergrund. Das ist bewusst so: Diese Prüfung
ist der einzige Moment, in dem die Anwendung von sich aus ins Netz geht, und der
Nutzer soll ihn auslösen, statt ihn hinzunehmen.

### Einmalig einzurichten

1. **Adresse.** Sie steht in `src-tauri/tauri.conf.json` unter
   `plugins.updater.endpoints` und zeigt auf
   `https://github.com/saschafo/gesture-timetrack`. Für ein anderes Repository:

   ```bash
   npm run repo:set konto/repository
   ```

2. **Signaturschlüssel.** Ein Schlüsselpaar wurde bereits erzeugt:

   | Datei | Zweck |
   |---|---|
   | `~/.tauri/gesture-timetrack.key` | privat – **niemals** ins Repo, gut sichern |
   | `~/.tauri/gesture-timetrack.key.pub` | öffentlich – steht bereits in `tauri.conf.json` |

   Der Schlüssel wurde ohne Passwort erzeugt. Vor der ersten öffentlichen
   Veröffentlichung besser mit Passwort neu erzeugen und den öffentlichen Teil
   in der Konfiguration ersetzen:

   ```bash
   npx tauri signer generate -w ~/.tauri/gesture-timetrack.key
   ```

   Geht der private Schlüssel verloren, lassen sich **keine** Updates mehr
   ausliefern, die bestehende Installationen annehmen – dann bleibt nur ein
   neuer Installer von Hand.

3. **Secrets im Repository** hinterlegen (*Settings → Secrets and variables →
   Actions*):

   | Secret | Inhalt |
   |---|---|
   | `TAURI_SIGNING_PRIVATE_KEY` | Inhalt der privaten Schlüsseldatei (eine Zeile) |
   | `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Passwort, sofern gesetzt |

   Die Schlüsseldatei ist **eine einzige Base64-Zeile ohne Zeilenumbruch**. Hängt
   beim Einfügen ein Umbruch daran, steht die Polsterung `==` nicht mehr am Ende
   und das Signieren bricht ab (`failed to decode base64 secret key`). Der
   Workflow putzt den Wert deshalb selbst; zum Kopieren ohne Umbruch:

   ```bash
   pbcopy < ~/.tauri/gesture-timetrack.key
   ```

### Veröffentlichen

```bash
npm run version:set 0.2.0
git commit -am "Version 0.2.0"
git tag -a v0.2.0 -m "Version 0.2.0"   # -a ist wichtig, siehe unten
git push --follow-tags
```

Das `-a` macht einen **annotierten** Tag daraus. `git push --follow-tags`
überträgt ausschließlich solche - ein einfaches `git tag v0.2.0` bliebe still
liegen, und der Release-Workflow würde nie starten. Wer lieber ohne `-a`
arbeitet, muss den Tag ausdrücklich pushen: `git push origin v0.2.0`.

Der Workflow [.github/workflows/release.yml](../.github/workflows/release.yml)
baut daraufhin macOS (Universal) und Windows, führt beide Testsuiten aus und legt
einen **Release-Entwurf** an – inklusive `latest.json`. Erst wenn der Entwurf
veröffentlicht wird, sehen die installierten Anwendungen das Update.
