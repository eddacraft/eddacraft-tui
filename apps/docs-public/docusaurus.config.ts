import { themes as prismThemes } from 'prism-react-renderer';
import type { Config } from '@docusaurus/types';
import type * as Preset from '@docusaurus/preset-classic';

const config: Config = {
  title: 'eddacraft',
  tagline: 'The forge for governed AI-assisted work',
  favicon: 'img/favicon.svg',

  future: {
    v4: true,
  },

  url: 'https://docs.eddacraft.ai',
  baseUrl: '/',

  organizationName: 'eddacraft',
  projectName: 'docs-public',

  // Cross-app links (e.g. to /anvil/overview) resolve at runtime via the
  // docs-shell proxy but are unresolvable at individual-app build time.
  onBrokenLinks: 'log',
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
        docs: false,
        blog: {
          showReadingTime: true,
          feedOptions: {
            type: ['rss', 'atom'],
            xslt: true,
          },
          onInlineTags: 'warn',
          onInlineAuthors: 'warn',
          onUntruncatedBlogPosts: 'warn',
        },
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
        id: 'aps',
        path: '../../docs/public/aps',
        routeBasePath: 'aps',
        sidebarPath: './sidebars/aps.ts',
      },
    ],
    [
      '@docusaurus/plugin-content-docs',
      {
        id: 'kindling',
        path: '../../docs/public/kindling',
        routeBasePath: 'kindling',
        sidebarPath: './sidebars/kindling.ts',
      },
    ],
    [
      '@docusaurus/plugin-content-docs',
      {
        id: 'edda-stack',
        path: '../../docs/public/edda-stack',
        routeBasePath: 'edda-stack',
        sidebarPath: './sidebars/edda-stack.ts',
      },
    ],
  ],

  themeConfig: {
    image: 'img/eddacraft-social-card.png',
    colorMode: {
      defaultMode: 'dark',
      disableSwitch: false,
      respectPrefersColorScheme: true,
    },
    navbar: {
      title: 'eddacraft',
      items: [
        {
          label: 'APS',
          to: '/aps/overview',
          position: 'left',
        },
        {
          label: 'Kindling',
          to: '/kindling/overview',
          position: 'left',
        },
        {
          label: 'Blog',
          to: '/blog',
          position: 'left',
        },
        {
          href: 'https://github.com/eddacraft',
          label: 'GitHub',
          position: 'right',
        },
      ],
    },
    footer: {
      style: 'dark',
      links: [
        {
          title: 'Docs',
          items: [
            { label: 'APS', to: '/aps/overview' },
            { label: 'Kindling', to: '/kindling/overview' },
          ],
        },
        {
          title: 'Community',
          items: [
            { label: 'GitHub', href: 'https://github.com/eddacraft' },
            { label: 'Releases', href: 'https://github.com/eddacraft/anvil-001/releases' },
          ],
        },
        {
          title: 'More',
          items: [{ label: 'Blog', to: '/blog' }],
        },
      ],
      copyright: `Copyright © ${new Date().getFullYear()} eddacraft, Inc. Built with Docusaurus.`,
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
