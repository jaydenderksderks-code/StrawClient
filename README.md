# StrawClient

A lightweight, CRT-inspired Minecraft Java launcher built with Tauri 2, Rust, React and TypeScript.

## Current status

The first repository milestone is the launcher shell and live Minecraft version catalogue. The catalogue is read from Mojang's official version manifest, so it includes every entry Mojang publishes: releases, snapshots, old beta and old alpha versions.

**Not implemented yet:** Microsoft sign-in, downloading game files, loader installation, mod downloads and launching the game. The UI does not pretend those operations are already functional.

## Development

Requirements: Node.js 20+, npm, Rust stable, and the native prerequisites for Tauri 2 on your OS.

```bash
npm install
npm run tauri dev
```

For a frontend-only preview:

```bash
npm run dev
```

## Design goals

- Low idle overhead; no Electron runtime.
- Network and filesystem work belongs in the Rust backend, not UI render loops.
- Per-instance game directories and isolated mod sets.
- Mojang's version manifest is the source of truth for version choices.
- Fabric, Forge, NeoForge and Quilt support planned through per-instance loader profiles.

## License

MIT. Minecraft is a trademark of Mojang Studios. StrawClient is an independent, unofficial project and is not affiliated with Mojang or Microsoft.
