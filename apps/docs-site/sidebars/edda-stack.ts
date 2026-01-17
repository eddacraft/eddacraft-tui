import type { SidebarsConfig } from '@docusaurus/plugin-content-docs';

const sidebars: SidebarsConfig = {
  eddaStackSidebar: [
    'overview',
    {
      type: 'category',
      label: 'Components',
      collapsed: false,
      items: ['components/kindling', 'components/ember', 'components/edda'],
    },
    {
      type: 'doc',
      id: 'design-principles',
      label: 'Design Principles',
    },
    {
      type: 'doc',
      id: 'roadmap',
      label: 'Roadmap',
    },
  ],
};

export default sidebars;
