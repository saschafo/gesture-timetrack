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

**Vor dem Veröffentlichen prüfen:** Auf den Bildern stehen echte Projektnamen und
Zeiten. Für die README lohnt es sich, vorher ein paar neutrale Beispielprojekte
anzulegen.
