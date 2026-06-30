import type { ItemEvent, SummaryEvent } from "./common";

export interface CleanSection {
  name: string;
  items: CleanItem[];
  totalSizeKb: number;
  status: "scanning" | "cleaned" | "nothing_to_clean";
}

export interface CleanItem {
  description: string;
  section: string;
  size_kb: number;
  size_human: string;
  status: ItemEvent["status"];
}

export interface CleanResult {
  sections: CleanSection[];
  summary: SummaryEvent | null;
  totalSizeKb: number;
  totalItems: number;
}
