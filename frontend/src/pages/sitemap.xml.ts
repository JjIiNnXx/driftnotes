import type { APIRoute } from 'astro';
import { getPosts } from '../lib/api';

export const GET: APIRoute = async ({ site }) => {
  const { items } = await getPosts({ pageSize: 100 });
  const origin = site?.origin || 'http://localhost:4321';
  const pages = ['', '/posts', '/tags', '/archive', '/about'];
  const urls = [
    ...pages.map(path => `<url><loc>${origin}${path}</loc></url>`),
    ...items.map(post => `<url><loc>${origin}/posts/${post.slug}</loc><lastmod>${post.updated_at}</lastmod></url>`),
  ];
  return new Response(`<?xml version="1.0" encoding="UTF-8"?><urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">${urls.join('')}</urlset>`, {
    headers: { 'Content-Type': 'application/xml; charset=utf-8', 'Cache-Control': 'public, max-age=300' },
  });
};
