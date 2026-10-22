export const settingsTabs=['general','recognition','queue','storage','projects']as const;
export type SettingsTab=typeof settingsTabs[number];
