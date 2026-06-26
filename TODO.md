# TODO

## FIXME BUG: Gateway surfaced `[[gateway.servers]]`-Addon-Tools nicht im stdio-MCP

**Status:** offen — blockiert die Live-Gates #4 (`addon_roundtrip`) und #5 (Delegation).
**Umgebung:** lean-ctx 3.8.13 (official) als stdio-MCP-Server; lean-md addon v0.1.0.
**Entdeckt:** 2026-06-26 (während lmd-v2-Finalisierung, Task 3/4).

### Symptom

Nach `cargo install --path .` + `lean-ctx addon add ./lean-ctx-addon.toml` ist das
Addon-Tool `lean-md::ctx_md_render` (und `lean-md::ctx_md_check`) auf **keinem**
erreichbaren Weg im Live-Katalog des laufenden stdio-MCP-Servers:

- `ctx_call name="lean-md::ctx_md_render"` → `MCP error -32602: Unknown tool`
- `ctx_call name="ctx_md_render"` (bare) → `Unknown tool`
- `ctx_discover_tools query="md_render"` → `No tools found`
- Claude-Code MCP `tools/list` (ToolSearch) → nicht vorhanden
- CLI `lean-ctx call ctx_md_render --project-root … --json …` → `unknown tool`
  (und `lean-ctx call ctx_read …` → `-32603: session not available`)

### Was korrekt ist (also NICHT die Ursache)

- Addon installiert + enabled: `lean-ctx addon list` → `✓ lean-md v0.1.0 → gateway server lean-md (local)`;
  `lean-ctx addon info lean-md` → `installed (gateway server lean-md, local)`.
- Config-Eintrag korrekt: `~/.config/lean-ctx/config.toml` `[[gateway.servers]]`
  (`name="lean-md"`, `transport="stdio"`, `enabled=true`, `command="lean-md"`, `args=["mcp"]`).
- `[gateway.servers.capabilities]` stimmt mit dem Manifest überein
  (`network="none"`, `filesystem="read_write"`, `exec=["lean-ctx"]`, `env=[]`).
- `lean-md` ist in `shell_allowlist_extra` (spawnbar).
- `[mcp].sha256` leer → community-tier, Gateway dürfte spawnen.
- **Funktion selbst ist korrekt:** direkter `lean-md mcp` JSON-RPC
  (`tools/call ctx_md_render`) rendert **byte-identisch** zu `lean-md render`
  (#4 funktional bestätigt — exakt der Spawn `command="lean-md" args=["mcp"]`,
  den der Gateway nutzen würde).

### Root-Cause (belegt via debug_log)

Mit `debug_log=true` + Server-Neustart + `ctx_call lean-md::ctx_md_render`:
das Debug-Log zeigt **nur** den fehlgeschlagenen Routing-Call
(`ERROR -32602 Unknown tool`) und **keinerlei** Gateway-Spawn-Versuch für lean-md
(kein Spawn, kein Capability-Check, kein Fehler).

∴ Der **stdio**-MCP-Server konsumiert `[[gateway.servers]]` gar nicht — er
spawnt/proxied externe Addon-Server nicht in seinen Live-Tool-Katalog. Der in
`doctor` gemeldete „ctx_call gateway" ist rein der lean-ctx-**interne**
non-core-Tool-Router, nicht ein Proxy für externe Addon-Server.

**Hypothese (offen):** Addon-Proxying greift evtl. nur unter `lean-ctx serve`
(HTTP-Gateway), nicht im stdio-MCP — oder es ist in 3.8.13 stdio ein
unimplementierter/fehlender Pfad.

### Repro

1. `cargo install --path /home/tholo/Scripts/lean-md`
2. `lean-ctx addon add /home/tholo/Scripts/lean-md/lean-ctx-addon.toml`
3. MCP-Server neu starten (`lean-ctx config apply`, dann Client reconnect)
4. `ctx_call name="lean-md::ctx_md_render" {"path":"…lmd.md"}` → `Unknown tool`

### Nächste Schritte

- [ ] In lokalem lean-ctx-Build prüfen: spawnt/proxied der **stdio**-MCP-Server
      `[[gateway.servers]]`? (vs. nur `lean-ctx serve` HTTP)
- [ ] Falls stdio-Proxying fehlt: implementieren ODER `addon add` muss klar
      kommunizieren, dass Addon-Tools nur im HTTP-`serve`-Gateway sichtbar sind.
- [ ] Hinweis: laufender Daemon/Server muss vom **neuen** Binary neu starten
      (`lean-ctx stop` + reconnect); systemd-Unit ggf. auf neues Binary zeigen
      lassen — sonst kommt der alte Daemon zurück.
- [ ] Nach Fix: Live-Gates #4 (`addon_roundtrip --run-ignored ignored-only`)
      und #5 (Delegation) real grün fahren; Spec §5.1.2 mit echtem Nachweis
      aktualisieren.

### Verweise

- Spec: `docs/lean-md/specs/2026-06-26-lean-md-standalone-addon-design.md` §5.1.1/§5.1.2
- Test: `tests/addon_roundtrip.rs` (`#[ignore]`, nutzt `lean-ctx call ctx_md_render`)
- Manifest: `lean-ctx-addon.toml`
