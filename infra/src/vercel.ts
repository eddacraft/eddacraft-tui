import * as pulumi from '@pulumi/pulumi';
import { VercelApp } from './components/vercel-app.js';

const config = new pulumi.Config('vercel-apps');
const gitRepo = 'EddaCraft/anvil-001';

const websiteDatabaseUrl = config.getSecret('website-database-url');
const unosendApiKey = config.getSecret('unosend-api-key');

// IAC-003: Website (Next.js)
export const website = new VercelApp('website', {
  name: 'website',
  framework: 'nextjs',
  rootDirectory: 'apps/website',
  gitRepo,
  domains: ['eddacraft.ai'],
  envVars: {
    ...(websiteDatabaseUrl ? { DATABASE_URL: websiteDatabaseUrl } : {}),
    ...(unosendApiKey ? { UNOSEND_API_KEY: unosendApiKey } : {}),
  },
});

// IAC-004: Docs Site (Docusaurus)
export const docsSite = new VercelApp('docs-site', {
  name: 'docs-site',
  framework: 'docusaurus-2',
  rootDirectory: 'apps/docs-site',
  gitRepo,
  domains: ['docs.eddacraft.ai'],
});
