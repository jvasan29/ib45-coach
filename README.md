# IB 45 Coach

A private, local-first Windows accountability coach for one IB Diploma student. The app is rooted at `D:\IB45Coach` and combines subject evidence, official IB point structure, an adaptive action queue, a searchable D-drive resource library, Google Calendar, Windows notifications and optional hybrid AI.

## What works

- Guided setup for six subjects, 3–4 HL choices, sleep/capacity constraints and TOK/EE/CAS.
- SQLCipher-encrypted SQLite records and encrypted database backups on D.
- Assessment evidence, error diagnoses, confidence-aware subject projections and the official TOK/EE core matrix.
- Deterministic task ranking based on urgency, recurring weakness, effort and expected impact.
- Background file hashing, deduplication, PDF/DOCX/text extraction, optional Tesseract OCR and SQLite full-text search.
- OpenAI Responses API routing with schema-constrained output and `store: false`; Ollama `qwen3:4b` fallback.
- Installed-app Google OAuth, incremental Calendar sync and explicit per-calendar read/edit authorization.
- Windows tray behavior, optional autostart, upcoming-action notifications and local audit history.

## First launch

Double-click `Install-IB45Coach-on-D.cmd`. It redirects installer temporary files to D (important on machines with a full C drive), opens the generated NSIS package, and prompts you to choose `D:\IB45Coach\app` as the install directory. Complete onboarding before starting the full resource index.

Optional integrations live under **Settings & privacy**:

- OpenAI: paste an API key. The key is stored in Windows Credential Manager.
- Google Calendar: create an OAuth client of type **Desktop app**, then paste its client ID and optional secret.
- Offline AI: install Ollama and run `ollama pull qwen3:4b`.
- OCR: install Tesseract and ensure `tesseract.exe` is on `PATH`.

## Development

```powershell
npm install
npm test
npm run build
npm run tauri dev
npm run tauri build
```

The vendored SQLCipher/OpenSSL build needs a complete Perl installation on Windows. The implementation machine uses Strawberry Perl.

## Safety boundaries

- A 45 is an aspiration, never a guarantee.
- AI proposals cannot directly mutate academic records or calendars.
- The resource index never modifies or deletes original study files.
- Calendar auto-editing is limited to explicitly authorized calendars; attendee-bearing events are excluded.
- Generated assessed-work material is labeled with an IB academic-integrity warning and its output history is retained locally.
