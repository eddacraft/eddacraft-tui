import { themes as prismThemes } from 'prism-react-renderer';
import type { Config } from '@docusaurus/types';
import type * as Preset from '@docusaurus/preset-classic';

const config: Config = {
  title: 'EddaCraft',
  tagline: 'The forge for governed AI-assisted work',
  favicon: 'img/favicon.ico',

  future: {
    v4: true,
  },

  url: 'https://docs.eddacraft.ai',
  baseUrl: '/',

  organizationName: 'EddaCraft',
  projectName: 'eddacraft-docs',

  onBrokenLinks: 'throw',
  onBrokenMarkdownLinks: 'warn',

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
          editUrl: 'https://github.com/EddaCraft/anvil-001/tree/main/website/',
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
    //     path: 'docs/start-here',
    //     routeBasePath: 'docs/start-here',
    //     sidebarPath: './sidebars/start-here.ts',
    //     editUrl: 'https://github.com/EddaCraft/anvil-001/tree/main/website/',
    //   },
    // ],
    // Anvil - primary product
    [
      '@docusaurus/plugin-content-docs',
      {
        id: 'anvil',
        path: 'docs/anvil',
        routeBasePath: 'docs/anvil',
        sidebarPath: './sidebars/anvil.ts',
        editUrl: 'https://github.com/EddaCraft/anvil-001/tree/main/website/',
      },
    ],
    // APS - OSS spec
    [
      '@docusaurus/plugin-content-docs',
      {
        id: 'aps',
        path: 'docs/aps',
        routeBasePath: 'docs/aps',
        sidebarPath: './sidebars/aps.ts',
        editUrl: 'https://github.com/EddaCraft/anvil-001/tree/main/website/',
      },
    ],
    // DISABLED for go-live: Kindling - OSS memory capture
    // [
    //   '@docusaurus/plugin-content-docs',
    //   {
    //     id: 'kindling',
    //     path: 'docs/kindling',
    //     routeBasePath: 'docs/kindling',
    //     sidebarPath: './sidebars/kindling.ts',
    //     editUrl: 'https://github.com/EddaCraft/anvil-001/tree/main/website/',
    //   },
    // ],
    // DISABLED for go-live: Edda Stack - placeholder/roadmap
    // [
    //   '@docusaurus/plugin-content-docs',
    //   {
    //     id: 'edda-stack',
    //     path: 'docs/edda-stack',
    //     routeBasePath: 'docs/edda-stack',
    //     sidebarPath: './sidebars/edda-stack.ts',
    //     editUrl: 'https://github.com/EddaCraft/anvil-001/tree/main/website/',
    //   },
    // ],
  ],

  themeConfig: {
    image: 'img/eddacraft-social-card.png',
    colorMode: {
      defaultMode: 'dark',
      disableSwitch: false,
      respectPrefersColorScheme: true,
    },
    navbar: {
      title: 'EddaCraft',
      logo: {
        alt: 'EddaCraft Logo',
        src: 'img/logo.svg',
        srcDark: 'img/logo-dark.svg',
      },
      items: [
        {
          label: 'Anvil',
          to: '/docs/anvil/overview',
          position: 'left',
        },
        {
          label: 'APS',
          to: '/docs/aps/overview',
          position: 'left',
        },
        {
          label: 'Blog',
          to: '/blog',
          position: 'left',
        },
        // Right side
        {
          href: 'https://github.com/EddaCraft',
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
            { label: 'Anvil', to: '/docs/anvil/overview' },
            { label: 'APS', to: '/docs/aps/overview' },
          ],
        },
        {
          title: 'Docs',
          items: [
            { label: 'Anvil Overview', to: '/docs/anvil/overview' },
            { label: 'Anvil Quickstart', to: '/docs/anvil/quickstart' },
            { label: 'APS Spec', to: '/docs/aps/spec/taxonomy' },
          ],
        },
        {
          title: 'Community',
          items: [
            {
              label: 'GitHub',
              href: 'https://github.com/EddaCraft',
            },
            {
              label: 'Releases',
              href: 'https://github.com/EddaCraft/anvil-001/releases',
            },
          ],
        },
        {
          title: 'More',
          items: [{ label: 'Blog', to: '/blog' }],
        },
      ],
      copyright: `Copyright © ${new Date().getFullYear()} EddaCraft. Built with Docusaurus.`,
    },
    prism: {
      theme: prismThemes.github,
      darkTheme: prismThemes.dracula,
      additionalLanguages: ['bash', 'json', 'yaml', 'typescript'],
    },
    tableOfContents: {
      minHeadingLevel: 2,
      maxHeadingLevel: 4,
    },
  } satisfies Preset.ThemeConfig,
};

export default config;
