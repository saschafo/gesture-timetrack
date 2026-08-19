# Screenshots

Aufgenommen wird **fensterweise**, nicht als Bildschirmausschnitt – so kann
nichts anderes vom Schreibtisch ins Bild geraten:

```bash
npm run tauri:dev            # Anwendung starten
npm run shot hauptfenster    # Hauptfenster aufnehmen
```

Weitere Fenster brauchen einen Auslöser, weil sie nur zeitweise sichtbar sind:

| Bild | Vorbereitung | Befehl |
|---|---|---|
| `hauptfenster.png` | – | `npm run shot hauptfenster` |
| `menueleiste.png` | Symbol in der Menüleiste anklicken | `npm run shot menueleiste -- --window Gesture` |
| `overlay.png` | Hotkey drücken, dann zügig aufnehmen | `npm run shot overlay -- --window Gestenerkennung` |

Die Bilder werden auf 1600 px Breite gerechnet. Nur macOS – unter Windows tut es
die Bildschirmaufnahme des Systems (`Win`+`Umschalt`+`S`).

## Vorschaubild für geteilte Links

`social-preview.png` (1280 × 640) ist das Bild, das LinkedIn, Facebook & Co.
anzeigen, wenn jemand die Repository-Adresse teilt. GitHub liest es **nicht**
selbst aus dem Repository – es muss einmalig hochgeladen werden:
*Settings → General → Social preview → Upload an image*.

Neu erzeugen, wenn sich die Oberfläche geändert hat:

```bash
npm run shot hauptfenster
sips --resampleHeight 552 docs/screenshots/hauptfenster.png --out /tmp/fit.png
sips --padToHeightWidth 640 1280 --padColor F4F5FA /tmp/fit.png   --out docs/screenshots/social-preview.png
```

**Vor dem Veröffentlichen prüfen:** Auf den Bildern stehen echte Projektnamen und
Zeiten. Für die README lohnt es sich, vorher ein paar neutrale Beispielprojekte
anzulegen.
