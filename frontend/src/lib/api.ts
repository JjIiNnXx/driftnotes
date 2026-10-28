export interface Tag {
  id: string;
  name: string;
  slug: string;
  post_count?: number;
}

export interface Post {
  id: string;
  title: string;
  slug: string;
  excerpt: string;
  content_html: string;
  content_markdown?: string;
  status: 'published' | 'draft' | 'archived';
  published_at: string | null;
  created_at: string;
  updated_at: string;
  version: number;
  cover_image: string | null;
  tags: Tag[];
}

export interface Pagination {
  page: number;
  page_size: number;
  total: number;
  total_pages: number;
}

export interface PostListData {
  items: Post[];
  pagination: Pagination;
}

export interface ArchiveGroup {
  year: number;
  months: {
    month: number;
    posts: Post[];
  }[];
}

const API_ORIGIN = import.meta.env.API_ORIGIN || 'http://localhost:8787';

export async function apiFetch<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(`${API_ORIGIN}${path}`, {
    ...init,
    headers: { Accept: 'application/json', ...init?.headers },
  });
  if (!response.ok) {
    const body = await response.json().catch(() => null);
    throw new Error(body?.error?.message || `API 请求失败（${response.status}）`);
  }
  return response.json() as Promise<T>;
}

export async function getPosts(params: { page?: number; pageSize?: number; tag?: string; q?: string } = {}) {
  const search = new URLSearchParams();
  search.set('page', String(params.page || 1));
  search.set('page_size', String(params.pageSize || 20));
  if (params.tag) search.set('tag', params.tag);
  if (params.q) search.set('q', params.q);
  const result = await apiFetch<{ data: PostListData }>(`/api/posts?${search}`);
  return result.data;
}

export async function getPost(slug: string) {
  const result = await apiFetch<{ data: { post: Post; previous: Post | null; next: Post | null } }>(`/api/posts/${encodeURIComponent(slug)}`);
  return result.data;
}

export async function getTags() {
  const result = await apiFetch<{ data: Tag[] }>('/api/tags');
  return result.data;
}

export async function getArchive() {
  const result = await apiFetch<{ data: ArchiveGroup[] }>('/api/archive');
  return result.data;
}
