# Übersetzungen / Translations

Dieses Projekt verwendet [i18next](https://www.i18next.com/) für Internationalisierung und [inlang/Sherlock](https://inlang.com/) für die Verwaltung der Übersetzungen.

## Verwendung

### In Code

```tsx
import { useI18n } from '@/hooks/useI18n'

function MyComponent() {
  const { t } = useI18n()
  
  return <div>{t('common.loading')}</div>
}
```

### Übersetzungsstruktur

Die Übersetzungen sind in JSON-Dateien organisiert:
- `de.json` - Deutsch (Basis-Sprache)
- `en.json` - Englisch

Die Struktur folgt Namespaces:
```json
{
  "common": {
    "loading": "Laden...",
    "save": "Speichern"
  },
  "profile": {
    "title": "Mein Profil"
  }
}
```

## Sherlock Extension

1. Installiere die [Sherlock i18n Inspector](https://marketplace.visualstudio.com/items?itemName=inlang.sherlock) Extension in VS Code
2. Die Extension zeigt Übersetzungen direkt im Code an
3. Neue Übersetzungen können mit einem Klick extrahiert werden
4. Fehlende Übersetzungen werden automatisch erkannt

## Neue Übersetzungen hinzufügen

1. Füge den Übersetzungsschlüssel in `de.json` hinzu
2. Füge die Übersetzung in `en.json` hinzu
3. Verwende `t('namespace.key')` im Code
4. Sherlock erkennt automatisch fehlende Übersetzungen

## Konfiguration

Die inlang-Konfiguration befindet sich in `.inlang/settings.json`.

