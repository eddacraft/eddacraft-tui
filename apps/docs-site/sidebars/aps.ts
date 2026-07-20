import type { SidebarsConfig } from '@docusaurus/plugin-content-docs';

const sidebars: SidebarsConfig = {
  apsSidebar: [
    {
      type: 'category',
      label: 'Start here',
      collapsed: false,
      items: ['overview', 'getting-started', 'workflow', 'terminology'],
    },
    {
      type: 'category',
      label: 'How-to guides',
      collapsed: false,
      items: ['installation', 'guides/ai-agents', 'guides/monorepo'],
    },
    {
      type: 'category',
      label: 'Concepts',
      collapsed: true,
      items: ['spec/taxonomy', 'spec/file-layout', 'spec/determinism'],
    },
    {
      type: 'category',
      label: 'Examples',
      collapsed: true,
      items: ['examples/minimal-plan', 'examples/multi-module', 'schemas/examples'],
    },
    {
      type: 'category',
      label: 'Reference',
      collapsed: true,
      items: ['tooling/validation', 'schemas/json-schema'],
    },
  ],
};

export default sidebars;
