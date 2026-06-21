import type { SidebarsConfig } from '@docusaurus/plugin-content-docs';

const sidebars: SidebarsConfig = {
  kindlingSidebar: [
    'overview',
    {
      type: 'category',
      label: 'Quickstart',
      collapsed: false,
      items: ['quickstart/install', 'quickstart/first-memory', 'quickstart/automatic-capture'],
    },
    {
      type: 'category',
      label: 'Core Concepts',
      collapsed: false,
      items: [
        'concepts/capsules',
        'concepts/observations',
        'concepts/storage',
        'concepts/retrieval',
      ],
    },
    {
      type: 'category',
      label: 'Adapters',
      collapsed: true,
      items: [
        'adapters/claude-code',
        'adapters/opencode',
        'adapters/pocketflow',
        'adapters/custom',
      ],
    },
    {
      type: 'category',
      label: 'Reference',
      collapsed: true,
      items: ['reference/formats', 'reference/cli', 'reference/config', 'reference/crates'],
    },
  ],
};

export default sidebars;
