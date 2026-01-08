# iOS App auf iPhone installieren

Es gibt mehrere Möglichkeiten, die `.ipa` Datei auf dein iPhone zu installieren:

## Methode 1: Über Xcode (Empfohlen)

### Voraussetzungen:
- iPhone per USB mit Mac verbunden
- Xcode installiert
- Apple Developer Account (kostenlos für Entwicklung)
- Code Signing in Xcode konfiguriert

### Schritte:

1. **Xcode-Projekt öffnen:**
   ```bash
   cd apps/tauri
   pnpm tauri ios build --open
   ```

2. **In Xcode:**
   - Wähle dein iPhone als Build-Target (oben in der Toolbar)
   - Klicke auf "Run" (▶️) oder drücke `Cmd+R`
   - Xcode baut die App und installiert sie automatisch auf dein iPhone

3. **Beim ersten Mal:**
   - Auf dem iPhone: Einstellungen → Allgemein → VPN & Geräteverwaltung
   - Vertraue deinem Entwickler-Zertifikat

## Methode 2: Über Finder (macOS Catalina+)

### Voraussetzungen:
- iPhone per USB mit Mac verbunden
- iPhone entsperrt
- "Diesem Computer vertrauen" auf dem iPhone bestätigt

### Schritte:

1. **iPhone mit Mac verbinden** (USB-Kabel)

2. **Finder öffnen** und iPhone in der Sidebar wählen

3. **App-Datei finden:**
   ```bash
   # Die .ipa Datei sollte hier sein:
   /Users/klank/src/my-movies/apps/tauri/src-tauri/gen/apple/build/arm64/My Movies.ipa
   ```

4. **In Finder:**
   - Gehe zum iPhone in der Sidebar
   - Scrolle zu "Dateien" oder "Apps"
   - Ziehe die `.ipa` Datei per Drag & Drop in den Finder
   - Oder: Rechtsklick auf iPhone → "Apps" → "App hinzufügen" → `.ipa` auswählen

## Methode 3: Über Apple Configurator 2

### Voraussetzungen:
- Apple Configurator 2 installiert (kostenlos im Mac App Store)
- iPhone per USB verbunden

### Schritte:

1. **Apple Configurator 2 öffnen**

2. **iPhone verbinden** (erscheint in der Liste)

3. **App hinzufügen:**
   - Klicke auf dein iPhone
   - Klicke auf "Apps" → "+" → "App hinzufügen"
   - Wähle die `.ipa` Datei aus

## Methode 4: Über TestFlight (für Beta-Tests)

### Voraussetzungen:
- Apple Developer Account ($99/Jahr)
- App Store Connect Account
- App bereits in App Store Connect hochgeladen

### Schritte:

1. **App in App Store Connect hochladen:**
   ```bash
   cd apps/tauri
   pnpm tauri ios build
   # Dann über Xcode: Product → Archive → Distribute App → App Store Connect
   ```

2. **TestFlight konfigurieren** in App Store Connect

3. **Tester hinzufügen** (intern oder extern)

4. **TestFlight App** auf dem iPhone installieren

## Methode 5: Über 3uTools oder ähnliche Tools

⚠️ **Nicht empfohlen** - Diese Tools können Sicherheitsrisiken darstellen.

## Troubleshooting

### "Untrusted Developer" Fehler:

1. Auf dem iPhone: Einstellungen → Allgemein → VPN & Geräteverwaltung
2. Finde dein Entwickler-Zertifikat
3. Tippe darauf und wähle "Vertrauen"

### "App konnte nicht installiert werden":

- Prüfe, ob Code Signing korrekt konfiguriert ist
- Prüfe, ob das Bundle Identifier korrekt ist
- Prüfe, ob Provisioning Profile gültig ist
- Prüfe Xcode-Konsole für Fehlermeldungen

### "Device not found":

- Prüfe USB-Verbindung
- Prüfe, ob iPhone entsperrt ist
- Prüfe, ob "Diesem Computer vertrauen" bestätigt wurde
- Versuche ein anderes USB-Kabel

## Schnellste Methode

Die schnellste Methode ist **Methode 1 (Xcode)**, da sie:
- Automatisch baut und installiert
- Code Signing automatisch verwaltet
- Fehler direkt anzeigt
- Debugging ermöglicht

## Alternative: AirDrop (wenn möglich)

Falls die `.ipa` Datei klein genug ist und beide Geräte AirDrop unterstützen:

1. `.ipa` Datei im Finder finden
2. Rechtsklick → Teilen → AirDrop
3. iPhone auswählen
4. Auf dem iPhone: Installieren

**Hinweis:** AirDrop funktioniert nur, wenn die App bereits signiert ist und das iPhone vertraut.

