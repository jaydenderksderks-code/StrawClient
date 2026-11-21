export type VersionType = "release" | "snapshot" | "old_beta" | "old_alpha";
export interface MinecraftVersion {
  id: string;
  type: VersionType;
  url: string;
  releaseTime: string;
}

interface MojangManifest {
  latest: { release: string; snapshot: string };
  versions: MinecraftVersion[];
}

const MANIFEST_URL = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
let cachedManifest: Promise<MojangManifest> | null = null;

/** Fetch once and share the in-flight request between callers. */
export function getVersionManifest(): Promise<MojangManifest> {
  if (!cachedManifest) {
    cachedManifest = fetch(MANIFEST_URL, { headers: { Accept: "application/json" } })
      .then((response) => {
        if (!response.ok) throw new Error(`Mojang manifest returned HTTP ${response.status}`);
        return response.json() as Promise<MojangManifest>;
      })
      .catch((error) => {
        cachedManifest = null;
        throw error;
      });
  }
  return cachedManifest;
}

export async function getVersions(): Promise<MinecraftVersion[]> {
  const manifest = await getVersionManifest();
  return manifest.versions;
}

export async function getLatestVersions() {
  const manifest = await getVersionManifest();
  return manifest.latest;
}
