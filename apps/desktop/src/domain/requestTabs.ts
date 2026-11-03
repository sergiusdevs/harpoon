import type { Exchange, Request } from "./types";
import type { RequestFailure } from "./requestFailure";
export type TabView = { draft: Request; dirty: boolean; exchange: Exchange | null; error: string | null; failure: RequestFailure | null };
export type RequestTabs = { opened: string[]; views: Record<string, TabView> };
export const emptyTabs = (): RequestTabs => ({ opened: [], views: {} });
const copy = <T>(value: T): T => JSON.parse(JSON.stringify(value));
export function activateTab(tabs: RequestTabs, previous: string | null, view: TabView | null, key: string, requests: Record<string, Request>): TabView | null {
  if (!requests[key]) return null;
  if (previous && view && requests[previous]) tabs.views[previous] = copy(view);
  if (!tabs.opened.includes(key)) tabs.opened.push(key);
  const cached = tabs.views[key];
  return cached ? { ...copy(cached), draft: copy(cached.dirty ? cached.draft : requests[key]) } : { draft: copy(requests[key]), dirty: false, exchange: null, error: null, failure: null };
}
export function remapTabs(tabs: RequestTabs, from: string, to: string, folder = false) {
  const rename = (key: string) => key === from || (folder && key.startsWith(from + "/")) ? to + key.slice(from.length) : key;
  tabs.opened = tabs.opened.map(rename);
  tabs.views = Object.fromEntries(Object.entries(tabs.views).map(([key, view]) => [rename(key), view]));
  return rename;
}
export function pruneTabs(tabs: RequestTabs, requests: Record<string, Request>) {
  tabs.opened = tabs.opened.filter((key) => key in requests);
  for (const key of Object.keys(tabs.views)) if (!(key in requests)) delete tabs.views[key];
}
