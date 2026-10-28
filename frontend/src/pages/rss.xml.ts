import type { APIRoute } from 'astro';
import { getPosts } from '../lib/api';

const escapeXml = (value: string) => value.replace(/[<>&'"]/g, char => ({ '<': '&lt;', '>': '&gt;', '&': '&amp;', "'": '&apos;', '"': '&quot;' })[char]!);

export const GET: APIRoute = async ({ site }) => {
  const { items } = await getPosts({ pageSize: 100 });
  const origin = site?.origin || 'http://localhost:4321';
  const xml = `<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0"><channel><title>漂流笔记</title><link>${origin}</link><description>记录技术、阅读与缓慢生活</description><language>zh-CN</language>
${items.map(post => `<item><title>${escapeXml(post.title)}</title><link>${origin}/posts/${post.slug}</link><guid isPermaLink="true">${origin}/posts/${post.slug}</guid><pubDate>${new Date(post.published_at!).toUTCString()}</pubDate><description>${escapeXml(post.excerpt)}</description></item>`).join('\n')}
</channel></rss>`;
  return new Response(xml, { headers: { 'Content-Type': 'application/rss+xml; charset=utf-8', 'Cache-Control': 'public, max-age=300' } });
};
