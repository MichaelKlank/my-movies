# iOS WebSocket Setup

## Problem

iOS blockiert standardmäßig alle nicht-HTTPS Verbindungen, einschließlich WebSocket-Verbindungen zu `localhost` oder `127.0.0.1`. Dies verhindert, dass die App eine WebSocket-Verbindung zum eingebetteten Server herstellen kann.

## Lösung

Die `Info.plist` muss App Transport Security (ATS) so konfigurieren, dass localhost-Verbindungen erlaubt sind.

## Konfiguration

Füge folgende Einstellungen zur `Info.plist` hinzu:

```xml
<key>NSAppTransportSecurity</key>
<dict>
    <key>NSAllowsLocalNetworking</key>
    <true/>
    <key>NSExceptionDomains</key>
    <dict>
        <key>localhost</key>
        <dict>
            <key>NSExceptionAllowsInsecureHTTPLoads</key>
            <true/>
            <key>NSIncludesSubdomains</key>
            <true/>
        </dict>
        <key>127.0.0.1</key>
        <dict>
            <key>NSExceptionAllowsInsecureHTTPLoads</key>
            <true/>
            <key>NSIncludesSubdomains</key>
            <true/>
        </dict>
    </dict>
</dict>
```

## Datei-Location

Die `Info.plist` befindet sich in:
```
apps/tauri/src-tauri/gen/apple/my-movies-tauri_iOS/Info.plist
```

## Wichtiger Hinweis

⚠️ **Diese Datei wird möglicherweise von Tauri bei jedem `tauri ios init` oder `tauri ios build` neu generiert.**

Falls die Einstellungen nach einem Build verschwinden, musst du sie erneut hinzufügen.

## Alternative: Manuelle Konfiguration in Xcode

1. Öffne das Xcode-Projekt:
   ```bash
   cd apps/tauri
   pnpm tauri ios build --open
   ```

2. Wähle das Projekt im Navigator aus
3. Wähle das Target `my-movies-tauri_iOS` aus
4. Gehe zum Tab "Info"
5. Füge die ATS-Einstellungen manuell hinzu:
   - `App Transport Security Settings` (Dictionary)
     - `Allow Arbitrary Loads in Web Content` = `NO`
     - `Allow Local Networking` = `YES`
     - `Exception Domains` (Dictionary)
       - `localhost` (Dictionary)
         - `Exception Allows Insecure HTTP Loads` = `YES`
         - `Includes Subdomains` = `YES`
       - `127.0.0.1` (Dictionary)
         - `Exception Allows Insecure HTTP Loads` = `YES`
         - `Includes Subdomains` = `YES`

## Testen

Nach der Konfiguration sollte die WebSocket-Verbindung funktionieren. Du kannst dies in der Browser-Konsole prüfen:

```javascript
// In der App-Konsole sollte erscheinen:
// "WebSocket connected"
```

Falls weiterhin Probleme auftreten, prüfe:
1. Ob der eingebettete Server läuft (Port 3000)
2. Ob die WebSocket-URL korrekt ist (`ws://127.0.0.1:3000/ws?token=...`)
3. Die Xcode-Konsole für Fehlermeldungen

