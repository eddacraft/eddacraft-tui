import { VercelApp } from './components/vercel-app.js';
import { getSecret } from './keyvault.js';

const gitRepo = 'EddaCraft/anvil-001';

const websiteDatabaseUrl = getSecret('website-database-url');
const resendApiKey = getSecret('resend-api-key');

// IAC-003: Website (Next.js)
export const website = new VercelApp('website', {
  name: 'website',
  framework: 'nextjs',
  rootDirectory: 'apps/website',
  gitRepo,
  domains: ['eddacraft.ai'],
  envVars: {
    DATABASE_URL: websiteDatabaseUrl,
    RESEND_API_KEY: resendApiKey,
  },
});

// IAC-005: Anvil API (Hono)
export const api = new VercelApp('anvil-api', {
  name: 'anvil-api',
  framework: 'hono',
  rootDirectory: 'apps/anvil-api',
  gitRepo,
  domains: ['api.eddacraft.ai'],
  envVars: {
    DATABASE_URL: websiteDatabaseUrl,
    RESEND_API_KEY: resendApiKey,
    ANVIL_ADMIN_KEY: getSecret('anvil-admin-key'),
    WAITLIST_RESEND_ADMIN_TOKEN: getSecret('waitlist-resend-admin-token'),
    ANVIL_CORS_ORIGINS: 'https://eddacraft.ai',
  },
});

// IAC-004: Docs Site (Docusaurus)
export const docsSite = new VercelApp('docs-site', {
  name: 'docs-site',
  framework: 'docusaurus-2',
  rootDirectory: 'apps/docs-site',
  gitRepo,
  domains: ['docs.eddacraft.ai'],
  extraWatchPaths: ['docs/public'],
});
