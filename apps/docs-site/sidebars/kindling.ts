import type { SidebarsConfig } from '@docusaurus/plugin-content-docs';

const sidebars: SidebarsConfig = {
  kindlingSidebar: [
    'overview',
    {
      type: 'category',
      label: 'Quickstart',
      collapsed: false,
      items: [
        'quickstart/install',
        'quickstart/create-capsule',
        'quickstart/write-observations',
        'quickstart/search-export',
      ],
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
      items: ['adapters/opencode', 'adapters/pocketflow', 'adapters/custom'],
    },
    {
      type: 'category',
      label: 'Commands',
      collapsed: true,
      items: ['commands/memory'],
    },
    {
      type: 'category',
      label: 'Reference',
      collapsed: true,
      items: ['reference/formats', 'reference/cli', 'reference/config'],
    },
  ],
};

export default sidebars;
