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

  markdown: {
    format: 'detect',
    hooks: {
      onBrokenMarkdownLinks: 'warn',
    },
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
    //   },
    // ],
    // anvil - primary product
    [
      '@docusaurus/plugin-content-docs',
      {
        id: 'anvil',
        path: '../../docs/public/anvil',
        routeBasePath: 'anvil',
        sidebarPath: './sidebars/anvil.ts',
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
      },
    ],
    // kindling - OSS memory capture
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
    // Beta - unlisted quickstart for beta testers (not in navbar/footer)
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
          label: 'anvil',
          to: '/anvil/overview',
          position: 'left',
        },
        {
          label: 'APS',
          to: '/aps/overview',
          position: 'left',
        },
        {
          label: 'kindling',
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
            { label: 'anvil', to: '/anvil/overview' },
            { label: 'APS', to: '/aps/overview' },
            { label: 'kindling', to: '/kindling/overview' },
          ],
        },
        {
          title: 'Docs',
          items: [
            { label: 'What anvil does', to: '/anvil/overview' },
            { label: 'Install anvil', to: '/anvil/quickstart' },
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
              href: 'https://github.com/eddacraft/anvil/releases',
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
