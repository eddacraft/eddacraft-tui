import { Hono } from 'hono';
import { cors } from 'hono/cors';
import { logger } from 'hono/logger';
import { auth } from './routes/auth.js';
import { admin } from './routes/admin.js';
import { waitlist } from './routes/waitlist.js';
import { rateLimiter } from './middleware/rate-limit.js';
import { getClient } from './db/client.js';

const app = new Hono().basePath('/api/v1');

app.use('*', logger());

// CORS: restrict to configured origins, or disable for admin routes if no UI exists
const allowedOrigins = process.env.ANVIL_CORS_ORIGINS
  ? process.env.ANVIL_CORS_ORIGINS.split(',').map((o) => o.trim())
  : [];

app.use(
  '*',
  cors({
    origin: allowedOrigins.length > 0 ? allowedOrigins : [],
    allowMethods: ['GET', 'POST', 'PUT', 'DELETE', 'OPTIONS'],
    allowHeaders: ['Content-Type', 'Authorization'],
    maxAge: 86400,
  })
);

app.use('*', rateLimiter());

app.get('/health', async (c) => {
  try {
    const sql = getClient();
    await sql`SELECT 1`;
    return c.json({ status: 'ok' });
  } catch {
    return c.json({ status: 'degraded', db: 'unreachable' }, 503);
  }
});

app.route('/auth', auth);
app.route('/admin', admin);
app.route('/waitlist', waitlist);

export default app;
