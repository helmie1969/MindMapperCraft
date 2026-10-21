# MindMapperCraft

Desktop-App zum Erstellen von MindMaps, geschrieben in Rust mit [egui/eframe](https://github.com/emilk/egui).
Bedienung und Workflow orientieren sich an Xmind: Themen werden per Tastatur angelegt,
Struktur und Formatierung stellst du in der rechten Seitenleiste ein.

> Status: erster Entwurf (Prototyp)

## Bauen und starten

Voraussetzungen unter Windows:

1. Rust über [rustup](https://rustup.rs) installieren (`winget install Rustlang.Rustup`)
2. Visual Studio (oder die *Build Tools for Visual Studio*) mit dem Workload
   **„Desktopentwicklung mit C++“** installieren, damit der MSVC-Linker `link.exe` verfügbar ist

```powershell
cargo run            # Debug-Build
cargo run --release  # optimierter Build ohne Konsolenfenster
```

## Bedienung

| Taste / Aktion              | Wirkung                                                    |
|-----------------------------|------------------------------------------------------------|
| **Tab** (oder Einfg)        | Unterthema zum ausgewählten Thema einfügen                 |
| **Enter**                   | Thema auf gleicher Ebene einfügen; beim Bearbeiten: übernehmen |
| **F2**, **Leertaste**, Doppelklick | Text des Themas bearbeiten                          |
| Einfach lostippen           | Text des ausgewählten Themas ersetzen                      |
| **Esc**                     | Bearbeitung abbrechen                                      |
| **Entf** / Rücktaste        | Thema mit allen Unterthemen löschen                        |
| Pfeiltasten                 | Auswahl zum nächstgelegenen Thema in Pfeilrichtung         |
| Strg+Z / Strg+Y             | Rückgängig / Wiederholen                                   |
| Strg+N / Strg+O / Strg+S    | Neu / Öffnen / Speichern (`.mmc`, JSON)                    |
| Strg+Mausrad                | Zoomen (zur Mausposition)                                  |
| Mausrad / Ziehen            | Ansicht verschieben                                        |

Neu angelegte Themen sind sofort im Bearbeitungsmodus – der Standardtext („Hauptthema“,
„Unterthema“) erscheint als Platzhalter und wird durch Tippen ersetzt. Mit
**Tab** während der Bearbeitung wird der Text übernommen und direkt ein Unterthema angelegt,
so lässt sich eine ganze Gliederung ohne Maus erfassen.

### Seitenleiste

- **Struktur** – MindMap-Typ der gesamten Karte, jederzeit umschaltbar:
  Mindmap (beidseitig), Logikdiagramm rechts/links, Organigramm, Baumdiagramm, Zeitleiste
- **Rahmen** – Form (Rechteck, abgerundet, Kapsel, Ellipse, unterstrichen, ohne),
  Füllfarbe, Rahmenfarbe, Rahmenstärke
- **Schrift** – Schriftart (installierte Windows-Schriften), Größe, fett, Farbe
- **Verbindungslinie** – Form (Kurve, Gerade, Winkel), Farbe, Stärke der Linie zum Elternthema
- **„Auch auf alle Unterthemen anwenden“** – überträgt nur die *geänderte* Eigenschaft auf
  den gesamten Zweig; ebenenspezifische Werte (z. B. unterschiedliche Schriftgrößen) bleiben erhalten
- **Karte** – Hintergrundfarbe, alle Formate zurücksetzen

## Architektur

```
src/
├── main.rs     Einstiegspunkt, Fenster-Optionen
├── app.rs      MindMapApp: Werkzeugleiste, Zeichenfläche, Tastatur-Workflow,
│               Undo/Redo, Laden/Speichern
├── sidebar.rs  Rechte Seitenleiste (Struktur-Kacheln, Formatierung)
├── model.rs    Datenmodell: MindMap, Node, NodeStyle, MapKind, BorderShape, LineStyle
├── layout.rs   Layout-Algorithmen je MindMap-Typ (berechnet Rechtecke + Kanten)
├── render.rs   Zeichnen von Themen, Linien und Struktur-Vorschaubildern
└── fonts.rs    Laden der Systemschriften für die Schriftauswahl
```

Der Ablauf pro Frame:

1. `raw_input_hook` fängt **Tab/Enter/Esc** ab, bevor egui sie für die Fokus-Navigation nutzt
   (außer ein Eingabefeld der Seitenleiste ist aktiv).
2. Die abgefangenen Tasten und sonstigen Kürzel werden auf das Modell angewendet.
3. `layout::compute` berechnet aus dem Modell die Positionen aller Themen in Weltkoordinaten –
   die Karte ist immer vollständig aus den Daten ableitbar, es gibt keine gespeicherten Positionen.
4. Der Canvas rechnet mit Zoom/Verschiebung in Bildschirmkoordinaten um, zeichnet erst Linien,
   dann Themen, und legt beim Bearbeiten ein rahmenloses Textfeld über das Thema.

Rückgängig arbeitet mit Schnappschüssen des gesamten Modells (für MindMap-Größen völlig ausreichend).
Gespeichert wird als lesbares JSON.

## Mögliche nächste Schritte

- Themen per Drag & Drop umhängen und umsortieren
- Zweige ein-/ausklappen
- Struktur pro Zweig (wie Xmind) statt nur für die ganze Karte; Fischgräten-Diagramm
- Mehrzeilige Themen (Umschalt+Enter), Notizen, Beziehungslinien, Markierungen/Icons
- Export als PNG/SVG/PDF und Markdown-Gliederung
- Themes (Farbschemata) und Vorlagen
- Rückfrage bei ungespeicherten Änderungen beim Schließen
