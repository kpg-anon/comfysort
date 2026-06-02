// Reactive settings store. Mirrors the backend `Settings` (config.toml) and
// persists every change. Frontend behaviors (sort defaults, confirm prompts,
// video preview) read from here; the collision policy is pushed to the backend.
import { api, DEFAULT_SETTINGS, type Settings } from "./api";

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

  /** Overlay visibility. */
  open = $state(false);
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

  toggleOpen() {
    this.open = !this.open;
  }
  close() {
    this.open = false;
  }
}

export const settings = new SettingsStore();
