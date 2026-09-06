/**
 * Typed fetch client for Indexium Public API v1.
 * Base URL via VITE_PUBLIC_API_URL (Vite / SvelteKit client env).
 */

export const BASE: string =
  (import.meta.env.VITE_PUBLIC_API_URL as string | undefined) ?? 'http://localhost:8080';

// ---------------------------------------------------------------------------
// Error
// ---------------------------------------------------------------------------

export class ApiError extends Error {
  status: number;
  constructor(status: number, message: string) {
    super(message);
    this.name = 'ApiError';
    this.status = status;
  }
}

// ---------------------------------------------------------------------------
// Types (mirror api-spec.md + backend DTOs)
// ---------------------------------------------------------------------------

export interface ModListItem {
  slug: string;
  name: string;
  summary: string | null;
  author: string;
  icon_url: string | null;
  game_versions: string[];
  loaders: string[];
  latest_version: string | null;
  download_url: string | null;
  updated_at: string;
}

export interface Pagination {
  page: number;
  limit: number;
  total: number;
  pages: number;
}

export interface ModListResponse {
  data: ModListItem[];
  pagination: Pagination;
}

export interface AuthorDto {
  login: string;
  avatar_url: string | null;
}

export interface VersionDto {
  version_number: string;
  game_versions: string[];
  loaders: string[];
  download_url: string;
  file_sha256: string;
  file_size: number | null;
  published_at: string;
}

export interface ModDetail {
  slug: string;
  name: string;
  summary: string | null;
  description: string | null;
  github_repo: string;
  author: AuthorDto;
  icon_url: string | null;
  verified: boolean;
  versions: VersionDto[];
  updated_at: string;
}

export interface DailyPoint {
  date: string;
  active_servers: number;
  active_players: number;
}

export interface Breakdown {
  mc_versions: Record<string, number>;
  loaders: Record<string, number>;
  os: Record<string, number>;
  java: Record<string, number>;
  custom: Record<string, Record<string, number>>;
}

export interface AnalyticsResponse {
  mod_slug: string;
  range: string;
  daily: DailyPoint[];
  breakdown: Breakdown;
}

// ---------------------------------------------------------------------------
// Params
// ---------------------------------------------------------------------------

export interface FetchModsParams {
  query?: string;
  gameVersion?: string;
  loader?: string;
  page?: number;
  limit?: number;
  sort?: 'relevance' | 'newest' | 'popular' | 'active_servers';
}

export type AnalyticsRange = '7d' | '30d' | '90d';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Build querystring, skipping empty / undefined values. */
export function buildQuery(params: Record<string, string | number | undefined | null>): string {
  const sp = new URLSearchParams();
  for (const [k, v] of Object.entries(params)) {
    if (v === undefined || v === null || v === '') continue;
    sp.set(k, String(v));
  }
  const qs = sp.toString();
  return qs ? `?${qs}` : '';
}

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const url = `${BASE}${path}`;
  const res = await fetch(url, {
    headers: { 'Content-Type': 'application/json', ...(init?.headers ?? {}) },
    ...init
  });
  if (!res.ok) {
    let message = res.statusText;
    try {
      const body = (await res.json()) as { message?: string; error?: string };
      message = body.message ?? body.error ?? message;
    } catch {
      // ignore json parse error
    }
    throw new ApiError(res.status, message);
  }
  return (await res.json()) as T;
}

// ---------------------------------------------------------------------------
// API functions
// ---------------------------------------------------------------------------

export async function fetchMods(params: FetchModsParams = {}): Promise<ModListResponse> {
  const qs = buildQuery({
    query: params.query,
    gameVersion: params.gameVersion,
    loader: params.loader,
    page: params.page,
    limit: params.limit,
    sort: params.sort
  });
  return request<ModListResponse>(`/api/v1/mods${qs}`);
}

export async function fetchMod(slug: string): Promise<ModDetail> {
  return request<ModDetail>(`/api/v1/mods/${encodeURIComponent(slug)}`);
}

export async function fetchAnalytics(
  slug: string,
  range: AnalyticsRange = '30d'
): Promise<AnalyticsResponse> {
  const qs = buildQuery({ range });
  return request<AnalyticsResponse>(`/api/v1/mods/${encodeURIComponent(slug)}/analytics${qs}`);
}

/** Stub for analytics ingestion (mod SDK → POST /analytics/submit). */
export async function submitAnalytics(payload: {
  mod_slug: string;
  server_uuid: string;
  metrics: {
    mc_version: string;
    loader: string;
    java_version: string;
    os: string;
    player_count: number;
    custom_charts?: Record<string, string>;
  };
}): Promise<{ status: string }> {
  return request<{ status: string }>('/api/v1/analytics/submit', {
    method: 'POST',
    body: JSON.stringify(payload)
  });
}
