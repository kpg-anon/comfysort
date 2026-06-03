// Reactive settings store. Mirrors the backend `Settings` (config.toml) and
// persists every change. Frontend behaviors (sort defaults, confirm prompts,
// video preview) read from here; the collision policy is pushed to the backend.
import { api, DEFAULT_SETTINGS, type Settings } from "./api";
import { openPath } from "@tauri-apps/plugin-opener";

class SettingsStore {
  collisionPolicy = $state(DEFAULT_SETTINGS.collisionPolicy);
  confirmFolderDelete = $state(DEFAULT_SETTINGS.confirmFolderDelete);
  confirmCrossDrive = $state(DEFAULT_SETTINGS.confirmCrossDrive);
  defaultSortField = $state(DEFAULT_SETTINGS.defaultSortField);
  defaultSortOrder = $state(DEFAULT_SETTINGS.defaultSortOrder);
  defaultFilter = $state(DEFAULT_SETTINGS.defaultFilter);
  videoAutoplay = $state(DEFAULT_SETTINGS.videoAutoplay);
  videoLoop = $state(DEFAULT_SETTINGS.videoLoop);
  videoMuted = $state(DEFAULT_SETTINGS.videoMuted);
  autoUpdateCheck = $state(DEFAULT_SETTINGS.autoUpdateCheck);
  theme = $state(DEFAULT_SETTINGS.theme);
  defaultInput = $state(DEFAULT_SETTINGS.defaultInput);
  defaultOutput = $state(DEFAULT_SETTINGS.defaultOutput);

  /** Overlay visibility. */
  open = $state(false);
  /** Sort-target editor popup visibility (opened from the Settings panel). */
  targetsOpen = $state(false);
  loaded = $state(false);

  private snapshot(): Settings {
    return {
      collisionPolicy: this.collisionPolicy,
      confirmFolderDelete: this.confirmFolderDelete,
      confirmCrossDrive: this.confirmCrossDrive,
      defaultSortField: this.defaultSortField,
      defaultSortOrder: this.defaultSortOrder,
      defaultFilter: this.defaultFilter,
      videoAutoplay: this.videoAutoplay,
      videoLoop: this.videoLoop,
      videoMuted: this.videoMuted,
      autoUpdateCheck: this.autoUpdateCheck,
      theme: this.theme,
      defaultInput: this.defaultInput,
      defaultOutput: this.defaultOutput,
    };
  }
  private apply(s: Settings) {
    this.collisionPolicy = s.collisionPolicy;
    this.confirmFolderDelete = s.confirmFolderDelete;
    this.confirmCrossDrive = s.confirmCrossDrive;
    this.defaultSortField = s.defaultSortField;
    this.defaultSortOrder = s.defaultSortOrder;
    this.defaultFilter = s.defaultFilter;
    this.videoAutoplay = s.videoAutoplay;
    this.videoLoop = s.videoLoop;
    this.videoMuted = s.videoMuted;
    this.autoUpdateCheck = s.autoUpdateCheck;
    this.theme = s.theme;
    this.defaultInput = s.defaultInput;
    this.defaultOutput = s.defaultOutput;
  }

  /** Load config.toml once at startup. */
  async load() {
    try {
      this.apply(await api.getSettings());
      await api.setCollisionPolicy(this.collisionPolicy);
    } catch {
      // keep defaults
    }
    this.loaded = true;
  }

  /** Set one field, persist to config.toml, and apply any backend side-effect. */
  async set<K extends keyof Settings>(key: K, value: Settings[K]) {
    (this as unknown as Settings)[key] = value;
    try {
      await api.setSettings(this.snapshot());
      if (key === "collisionPolicy") await api.setCollisionPolicy(this.collisionPolicy);
    } catch {
      // best-effort persistence
    }
  }

  /** Pick a default inbox/destination folder and persist it. */
  async pickDefault(which: "input" | "output") {
    const dir = await api.pickDirectory(
      which === "input" ? "Default inbox folder" : "Default destination root",
    );
    if (dir) await this.set(which === "input" ? "defaultInput" : "defaultOutput", dir);
  }
  async clearDefault(which: "input" | "output") {
    await this.set(which === "input" ? "defaultInput" : "defaultOutput", "");
  }

  /** Open config.toml in the OS default editor (ensures the file exists first). */
  async openConfigFile() {
    try {
      await api.setSettings(this.snapshot()); // write the file if it doesn't exist yet
      await openPath(await api.configFilePath());
    } catch {
      /* ignore */
    }
  }

  toggleOpen() {
    this.open = !this.open;
  }
  close() {
    this.open = false;
  }
  openTargets() {
    this.targetsOpen = true;
  }
  closeTargets() {
    this.targetsOpen = false;
  }
}

export const settings = new SettingsStore();
