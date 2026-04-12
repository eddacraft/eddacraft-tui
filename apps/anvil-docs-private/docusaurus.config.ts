import { themes as prismThemes } from 'prism-react-renderer';
import type { Config } from '@docusaurus/types';
import type * as Preset from '@docusaurus/preset-classic';

const config: Config = {
  title: 'Anvil Documentation',
  tagline: 'Governed AI-assisted development',
  favicon: 'img/favicon.svg',

  future: {
    v4: true,
  },

  url: 'https://docs.eddacraft.ai',
  baseUrl: '/anvil/',

  organizationName: 'EddaCraft',
  projectName: 'anvil-docs-private',

  onBrokenLinks: 'throw',
  onBrokenMarkdownLinks: 'warn',

  markdown: {
    format: 'detect',
  },

  i18n: {
    defaultLocale: 'en',
    locales: ['en'],
  },

  presets: [
    [
      'classic',
      {
        docs: {
          routeBasePath: '/',
          path: '../../docs/public/anvil',
          sidebarPath: './sidebars/anvil.ts',
        },
        blog: false,
        theme: {
          customCss: './src/css/custom.css',
        },
      } satisfies Preset.Options,
    ],
  ],

  plugins: [
    [
      '@docusaurus/plugin-content-docs',
      {
        id: 'beta',
        path: '../../docs/public/beta',
        routeBasePath: 'beta',
        sidebarPath: './sidebars/beta.ts',
      },
    ],
  ],

  themeConfig: {
    colorMode: {
      defaultMode: 'dark',
      disableSwitch: false,
      respectPrefersColorScheme: true,
    },
    navbar: {
      title: 'Anvil',
      items: [
        {
          label: 'Docs',
          to: '/',
          position: 'left',
        },
        {
          label: 'Beta',
          to: '/beta/quickstart',
          position: 'left',
        },
      ],
    },
    prism: {
      theme: prismThemes.github,
      darkTheme: prismThemes.vsDark,
      additionalLanguages: ['bash', 'json', 'yaml', 'typescript', 'toml', 'rust'],
    },
    tableOfContents: {
      minHeadingLevel: 2,
      maxHeadingLevel: 4,
    },
  } satisfies Preset.ThemeConfig,
};

export default config;
