type Parser<T> = { parse: (input: unknown) => T };

export async function requestParsed<T>(
  method: string,
  params: Record<string, unknown> | undefined,
  schema: Parser<T>
): Promise<T> {
  const res = await window.markbook.request(method, params || {});
  return schema.parse(res);
}

export type HealthState = {
  version: string;
  sidecar: boolean;
  workspacePath: string | null;
};

export type SidecarMeta = {
  running: boolean;
  pid: number | null;
  path: string | null;
};

export type Prefs = {
  recentWorkspaces: string[];
  lastWorkspace: string | null;
};

