import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getVersions, getLatestVersions, type MinecraftVersion, type VersionType } from "./lib/versions";

type Page = "terminal" | "instances" | "mods" | "settings" | "about";
type Filter = "all" | VersionType;

const menu: { id: Page; label: string; key: string }[] = [
  { id: "terminal", label: "Play Minecraft", key: "1" },
  { id: "instances", label: "Instances", key: "2" },
  { id: "mods", label: "Mods", key: "3" },
  { id: "settings", label: "Settings", key: "4" },
  { id: "about", label: "About", key: "5" }
];

function App() {
  const [page, setPage] = useState<Page>("terminal");
  const [menuIndex, setMenuIndex] = useState(0);
  const [versions, setVersions] = useState<MinecraftVersion[]>([]);
  const [latest, setLatest] = useState<{ release: string; snapshot: string } | null>(null);
  const [manifestState, setManifestState] = useState("CONTACTING MOJANG VERSION SERVICE...");
  const [filter, setFilter] = useState<Filter>("all");
  const [query, setQuery] = useState("");
  const [selectedVersion, setSelectedVersion] = useState("1.21.8");
  const [selectedLoader, setSelectedLoader] = useState("Fabric");
  const [instanceName, setInstanceName] = useState("Strawberry");
  const [status, setStatus] = useState("SYSTEM READY. SELECT AN ACTION.");
  const [rustPlatform, setRustPlatform] = useState("WEB PREVIEW");
  const [installing, setInstalling] = useState(false);

  async function loadVersions() {
    setManifestState("FETCHING OFFICIAL VERSION MANIFEST...");
    try {
      const list = await getVersions();
      const newest = await getLatestVersions();
      setVersions(list);
      setLatest(newest);
      setManifestState(`MANIFEST OK — ${list.length.toLocaleString()} VERSIONS FOUND`);
      if (!list.some((v) => v.id === selectedVersion)) setSelectedVersion(newest.release);
    } catch (error) {
      setManifestState(`NETWORK ERROR — ${String(error)}`);
    }
  }

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    listen<string>("install-progress", (event) => setStatus(event.payload))
      .then((fn) => { unlisten = fn; }).catch(() => {});
    return () => unlisten?.();
  }, []);

  async function installSelectedVersion() {
    setInstalling(true);
    setStatus(`STARTING INSTALLATION: ${selectedVersion}`);
    try {
      const path = await invoke<string>("install_version", { versionId: selectedVersion });
      setStatus(`INSTALL COMPLETE: ${selectedVersion} · ${path}`);
    } catch (error) {
      setStatus(`INSTALL FAILED: ${String(error)}`);
    } finally {
      setInstalling(false);
    }
  }

  useEffect(() => {
    loadVersions();
    invoke<string>("platform_info").then(setRustPlatform).catch(() => setRustPlatform("WEB PREVIEW — RUST BACKEND OFFLINE"));
  }, []);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement | null;
      if (target && ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName)) {
        if (event.key === "Escape") target.blur();
        return;
      }
      if (event.key === "ArrowDown") { event.preventDefault(); setMenuIndex((i) => (i + 1) % menu.length); }
      if (event.key === "ArrowUp") { event.preventDefault(); setMenuIndex((i) => (i + menu.length - 1) % menu.length); }
      if (event.key === "Enter") setPage(menu[menuIndex].id);
      if (/^[1-5]$/.test(event.key)) { const item = menu[Number(event.key) - 1]; setMenuIndex(Number(event.key) - 1); setPage(item.id); }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [menuIndex]);

  const filtered = useMemo(() => versions.filter((v) => {
    const matchesType = filter === "all" || v.type === filter;
    return matchesType && v.id.toLowerCase().includes(query.toLowerCase());
  }), [versions, filter, query]);

  const chooseVersion = (v: MinecraftVersion) => {
    setSelectedVersion(v.id);
    setStatus(`VERSION SELECTED: ${v.id} [${v.type.toUpperCase()}]`);
  };

  const renderPage = () => {
    if (page === "instances") return <section className="panel full">
      <h2>INSTANCE DIRECTORY</h2>
      <div className="instance-card"><span className="pixel-icon">■</span><div><b>{instanceName}</b><small>MINECRAFT {selectedVersion} / {selectedLoader.toUpperCase()}</small></div><span className="tag">ACTIVE PROFILE</span></div>
      <p className="muted">Each instance will have its own game directory, loader, settings, saves and mods. Instance creation and filesystem operations are not wired up yet.</p>
      <button onClick={() => setStatus("INSTANCE CREATION IS ON THE ROADMAP.")}>[ + NEW INSTANCE ]</button>
    </section>;
    if (page === "mods") return <section className="panel full">
      <h2>MOD MANAGEMENT</h2><p>ACTIVE INSTANCE: <b>{instanceName}</b></p>
      <div className="mod-grid">{["Fabric", "Forge", "NeoForge", "Quilt"].map((loader) => <button className={selectedLoader === loader ? "choice active" : "choice"} key={loader} onClick={() => { setSelectedLoader(loader); setStatus(`LOADER PROFILE SELECTED: ${loader.toUpperCase()}`); }}>{loader}</button>)}</div>
      <p className="muted">Loader selection is currently a profile preference only. Modrinth / CurseForge search, dependency resolution and installation are planned next.</p>
    </section>;
    if (page === "settings") return <section className="panel full"><h2>SYSTEM CONFIGURATION</h2><label>INSTANCE NAME<input value={instanceName} onChange={(e) => setInstanceName(e.target.value)} /></label><label>LOADER PROFILE<select value={selectedLoader} onChange={(e) => setSelectedLoader(e.target.value)}>{["Vanilla", "Fabric", "Forge", "NeoForge", "Quilt"].map((x) => <option key={x}>{x}</option>)}</select></label><p className="muted">Performance target: low idle CPU and memory use; asynchronous downloads; shared immutable libraries where compatible; isolated per-instance mods.</p></section>;
    if (page === "about") return <section className="panel full"><h2>ABOUT STRAWCLIENT</h2><p>STRAWCLIENT // RETRO CRT EDITION</p><p>VERSION 0.1.0 — EARLY DEVELOPMENT</p><p>Built with Tauri 2, Rust, React and TypeScript.</p><p className="muted">Independent community launcher. Not affiliated with Mojang or Microsoft.</p></section>;
    return <div className="workspace">
      <section className="panel play-panel">
        <h2>MAIN CONSOLE</h2>
        <pre className="landscape" aria-label="ASCII Minecraft landscape">{String.raw`       .       *              .       .
   _  /\_             _/\_          *
  / \/   \__      ___/    \_
 /        ▓▓\____/▓▓         \
/___▓▓__________▓▓________▓▓___\
   ░░   ░░   ░░    ░░  ░░
       [ ]                 /\
   ____|_|____       _____/  \____
  |  STRAW  |       |  CRAFT     |
  |_________|       |____________|
`}</pre>
        <h2>COMMAND MENU</h2>
        <div className="terminal-menu">{menu.map((item, i) => <button key={item.id} className={menuIndex === i ? "menu-item selected" : "menu-item"} onMouseEnter={() => setMenuIndex(i)} onClick={() => setPage(item.id)}><span>{menuIndex === i ? "▶" : " "}</span>{item.label}<kbd>{item.key}</kbd></button>)}</div>
        <p className="hint">↑ ↓ NAVIGATE &nbsp; ENTER SELECT &nbsp; 1–5 QUICK ACCESS</p>
      </section>
      <section className="panel version-panel">
        <h2>GAME PROFILE</h2>
        <div className="readout"><small>ACTIVE INSTANCE</small><strong>{instanceName || "UNNAMED"}</strong></div>
        <div className="readout"><small>SELECTED VERSION</small><strong>{selectedVersion}</strong></div>
        <div className="readout"><small>LOADER</small><strong>{selectedLoader.toUpperCase()}</strong></div>
        {latest && <div className="latest">LATEST RELEASE: {latest.release}<br/>LATEST SNAPSHOT: {latest.snapshot}</div>}
        <button className="refresh" disabled={installing} onClick={installSelectedVersion}>{installing ? "[ INSTALLING... ]" : "[ INSTALL SELECTED VERSION ]"}</button>
        <h2>VERSION CATALOGUE</h2>
        <input className="search" placeholder="SEARCH EVERY VERSION..." value={query} onChange={(e) => setQuery(e.target.value)} />
        <div className="filters">{(["all", "release", "snapshot", "old_beta", "old_alpha"] as Filter[]).map((f) => <button key={f} className={filter === f ? "filter active" : "filter"} onClick={() => setFilter(f)}>{f === "all" ? "ALL" : f.replace("_", " ").toUpperCase()}</button>)}</div>
        <div className="version-list" role="listbox" aria-label="Minecraft versions">
          {filtered.map((v) => <button role="option" aria-selected={selectedVersion === v.id} key={v.id} className={selectedVersion === v.id ? "version-row chosen" : "version-row"} onClick={() => chooseVersion(v)}><span>{v.id}</span><small>{v.type.toUpperCase()}</small></button>)}
          {filtered.length === 0 && <p className="muted">NO MATCHING VERSIONS.</p>}
        </div>
        <small className="muted">SHOWING ALL {filtered.length.toLocaleString()} MATCHES · MOJANG MANIFEST</small>
        <button className="refresh" onClick={loadVersions}>[ REFRESH VERSION LIST ]</button>
      </section>
    </div>;
  };

  return <main className="desk">
    <div className="desk-glow" />
    <div className="monitor">
      <div className="monitor-top"><span>STRAWBERRY COMPUTING CO. · MODEL SC-1988</span><span>COLOR DISPLAY 80×25</span></div>
      <div className="bezel">
        <div className="screen">
          <header className="screen-header"><span>✳ STRAWCLIENT ✳</span><small>V0.1.0</small></header>
          <div className="rule" />
          <div className="statusline"><span>● ONLINE</span><span>{rustPlatform}</span><span>JAVA EDITION</span></div>
          {renderPage()}
          <footer className="screen-footer"><span title={status}>{status}</span><span>{manifestState}</span></footer>
          <div className="crt-scanlines" />
        </div>
      </div>
      <div className="monitor-lower"><div className="sticker">🍓 STRAWCLIENT</div><div className="drive-slot" /><div className="power"><i /> POWER</div></div>
      <div className="monitor-foot" />
    </div>
    <div className="desk-caption">A LITTLE COMPUTER FOR A BLOCKY WORLD <span>✦</span> PRESS 1–5 TO NAVIGATE</div>
  </main>;
}

export default App;
