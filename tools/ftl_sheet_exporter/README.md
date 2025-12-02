# ftl_sheet_exporter

Small, standalone Rust tool to read a Google Sheet and regenerate `assets/locales/ja-JP/main.ftl`. This sits outside the game code; run it manually when you need to sync translations.

## Expected sheet layout
- Column A: `key`
- Column B: `value` (Japanese translation)
- Column C: optional `comment` (exported as `#` lines above the entry)
- First row can be headers (skipped by default)

## Configuration
Set these environment variables or pass CLI flags:
- `GOOGLE_SHEETS_SPREADSHEET_ID`: ID from the sheet URL
- `GOOGLE_SHEETS_RANGE`: A1 range (default `main!A:C`)
- `OUTPUT_FTL_PATH`: output file path (default `assets/locales/ja-JP/main.ftl`)
- `GOOGLE_CLIENT_SECRET_JSON` / `GOOGLE_APPLICATION_CREDENTIALS`: path to `clientsecret.json` for an installed app OAuth client
- `GOOGLE_OAUTH_TOKEN_STORE`: path to cache OAuth tokens (default `tools/ftl_sheet_exporter/.oauth_tokens.json`)
- `SHEETS_SKIP_HEADER`: `true`/`false` to drop the first row (default `true`)

The OAuth client must have access to the sheet (use an account you sign in with during the OAuth flow).

## Run
```bash
cargo run --manifest-path tools/ftl_sheet_exporter/Cargo.toml -- \
  --spreadsheet-id "<sheet_id>" \
  --range "main!A:C" \
  --output-path assets/locales/ja-JP/main.ftl \
  --client-secret ./clientsecret.json
```

If environment variables are set you can omit the flags:
```bash
GOOGLE_SHEETS_SPREADSHEET_ID=... \
GOOGLE_CLIENT_SECRET_JSON=./clientsecret.json \
cargo run --manifest-path tools/ftl_sheet_exporter/Cargo.toml -- --range "main!A:C"
```

The tool overwrites the target FTL file and prepends a generated notice. Multi-line cells are indented in Fluent style. The first run opens a browser to grant access; tokens are then cached for reuse.
