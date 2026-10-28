export function formatDate(value: string | null | undefined, style: 'short' | 'long' = 'long') {
  if (!value) return '未发布';
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat('zh-CN', style === 'long'
    ? { year: 'numeric', month: 'long', day: 'numeric' }
    : { year: 'numeric', month: '2-digit', day: '2-digit' }).format(date);
}

export function formatDateTime(value: string | null | undefined) {
  if (!value) return '—';
  const date = new Date(value);
  return new Intl.DateTimeFormat('zh-CN', {
    month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit', hour12: false,
  }).format(date);
}

export function readingTime(post: { content_html?: string; excerpt?: string }) {
  const length = (post.content_html || post.excerpt || '').replace(/<[^>]+>/g, '').length;
  return `${Math.max(1, Math.ceil(length / 450))} 分钟阅读`;
}

export const statusLabels: Record<string, string> = {
  published: '已发布',
  draft: '草稿',
  archived: '已归档',
};
