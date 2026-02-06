import { Hono } from 'hono';
import { cors } from 'hono/cors';
import { logger } from 'hono/logger';
import { auth } from './routes/auth.js';
import { admin } from './routes/admin.js';

const app = new Hono().basePath('/api/v1');

app.use('*', logger());
app.use('*', cors());

app.get('/health', (c) => {
  return c.json({ status: 'ok', timestamp: new Date().toISOString() });
});

app.route('/auth', auth);
app.route('/admin', admin);

export default app;
