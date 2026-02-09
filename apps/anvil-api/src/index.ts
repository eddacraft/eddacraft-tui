import { Hono } from 'hono';
import { cors } from 'hono/cors';
import { logger } from 'hono/logger';
import { auth } from './routes/auth.js';
import { admin } from './routes/admin.js';
import { rateLimiter } from './middleware/rate-limit.js';

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

app.get('/health', (c) => {
  return c.json({ status: 'ok', timestamp: new Date().toISOString() });
});

app.route('/auth', auth);
app.route('/admin', admin);

export default app;
