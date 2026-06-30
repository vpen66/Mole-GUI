import { load } from "@tauri-apps/plugin-store";
import type { Locale } from "./types";

const STORE_FILE = "settings.json";
const LOCALE_KEY = "locale";

export async function loadLocale(): Promise<Locale | null> {
  try {
    const store = await load(STORE_FILE);
    const value = await store.get<Locale>(LOCALE_KEY);
    return value ?? null;
  } catch {
    return null;
  }
}

export async function saveLocale(locale: Locale): Promise<void> {
  const store = await load(STORE_FILE);
  await store.set(LOCALE_KEY, locale);
  await store.save();
}

export function detectSystemLocale(): Locale {
  const lang = navigator.language.toLowerCase();
  if (lang.startsWith("zh")) return "zh";
  return "en";
}
