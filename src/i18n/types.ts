export type Locale = "en" | "zh";

export interface LocaleInfo {
  code: Locale;
  label: string;
  nativeLabel: string;
}

export const SUPPORTED_LOCALES: LocaleInfo[] = [
  { code: "en", label: "English", nativeLabel: "English" },
  { code: "zh", label: "Chinese", nativeLabel: "中文" },
];

export type Dictionary = Record<string, string>;
