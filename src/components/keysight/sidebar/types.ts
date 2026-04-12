export type SidebarTab =
  | "follow"
  | "cards"
  | "review"
  | "filter"
  | "export"
  | "details"
  | "context"
  | "note";

export interface SidebarTabItem {
  id: SidebarTab;
  label: string;
}
