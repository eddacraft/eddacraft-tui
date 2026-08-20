import type { SidebarsConfig } from '@docusaurus/plugin-content-docs';

const sidebars: SidebarsConfig = {
  kindlingSidebar: [
    {
      type: 'doc',
      id: 'overview',
      label: 'Explanation — Overview',
    },
    {
      type: 'category',
      label: 'Tutorial — Quickstart',
      collapsed: false,
      items: ['quickstart/install', 'quickstart/first-memory', 'quickstart/automatic-capture'],
    },
    {
      type: 'category',
      label: 'Explanation — Core concepts',
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
      label: 'How-to — Adapters',
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
