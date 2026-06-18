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
  projectName: 'eddacraft-docs',

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
        // Disable default docs - we use multi-instance
        docs: false,
        blog: {
          showReadingTime: true,
          feedOptions: {
            type: ['rss', 'atom'],
            xslt: true,
          },
          editUrl: 'https://github.com/eddacraft/anvil-001/tree/main/website/',
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
    // DISABLED for go-live: Start Here - folded into homepage
    // [
    //   '@docusaurus/plugin-content-docs',
    //   {
    //     id: 'start-here',
    //     path: '../../docs/public/start-here',
    //     routeBasePath: 'start-here',
    //     sidebarPath: './sidebars/start-here.ts',
    //     editUrl: 'https://github.com/eddacraft/anvil-001/tree/main/docs/public/start-here/',
    //   },
    // ],
    // Anvil - primary product
    [
      '@docusaurus/plugin-content-docs',
      {
        id: 'anvil',
        path: '../../docs/public/anvil',
        routeBasePath: 'anvil',
        sidebarPath: './sidebars/anvil.ts',
        editUrl: 'https://github.com/eddacraft/anvil-001/tree/main/docs/public/anvil/',
      },
    ],
    // APS - OSS spec
    [
      '@docusaurus/plugin-content-docs',
      {
        id: 'aps',
        path: '../../docs/public/aps',
        routeBasePath: 'aps',
        sidebarPath: './sidebars/aps.ts',
        editUrl: 'https://github.com/eddacraft/anvil-001/tree/main/docs/public/aps/',
      },
    ],
    // Kindling - OSS memory capture
    [
      '@docusaurus/plugin-content-docs',
      {
        id: 'kindling',
        path: '../../docs/public/kindling',
        routeBasePath: 'kindling',
        sidebarPath: './sidebars/kindling.ts',
        editUrl: 'https://github.com/eddacraft/anvil-001/tree/main/docs/public/kindling/',
      },
    ],
    [
      '@docusaurus/plugin-content-docs',
      {
        id: 'edda-stack',
        path: '../../docs/public/edda-stack',
        routeBasePath: 'edda-stack',
        sidebarPath: './sidebars/edda-stack.ts',
        editUrl: 'https://github.com/eddacraft/anvil-001/tree/main/docs/public/edda-stack/',
      },
    ],
    // Beta - unlisted quickstart for beta testers (not in navbar/footer)
    [
      '@docusaurus/plugin-content-docs',
      {
        id: 'beta',
        path: '../../docs/public/beta',
        routeBasePath: 'beta',
        sidebarPath: './sidebars/beta.ts',
        editUrl: 'https://github.com/eddacraft/anvil-001/tree/main/docs/public/beta/',
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
      logo: {
        alt: 'eddacraft Logo',
        src: 'img/logo.svg',
        srcDark: 'img/logo-dark.svg',
      },
      items: [
        {
          label: 'Anvil',
          to: '/anvil/overview',
          position: 'left',
        },
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
          label: 'Memory',
          to: '/edda-stack/overview',
          position: 'left',
        },
        {
          label: 'Blog',
          to: '/blog',
          position: 'left',
        },
        // Right side
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
          title: 'Products',
          items: [
            { label: 'Anvil', to: '/anvil/overview' },
            { label: 'APS', to: '/aps/overview' },
            { label: 'Kindling', to: '/kindling/overview' },
          ],
        },
        {
          title: 'Docs',
          items: [
            { label: 'Anvil Overview', to: '/anvil/overview' },
            { label: 'Anvil Quickstart', to: '/anvil/quickstart' },
            { label: 'APS Spec', to: '/aps/spec/taxonomy' },
          ],
        },
        {
          title: 'Community',
          items: [
            {
              label: 'GitHub',
              href: 'https://github.com/eddacraft',
            },
            {
              label: 'Releases',
              href: 'https://github.com/eddacraft/anvil-001/releases',
            },
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
