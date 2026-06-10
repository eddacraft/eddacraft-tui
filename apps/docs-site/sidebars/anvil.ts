import type { SidebarsConfig } from '@docusaurus/plugin-content-docs';

const sidebars: SidebarsConfig = {
  anvilSidebar: [
    {
      type: 'category',
      label: 'Overview',
      collapsed: false,
      items: ['overview', 'when-to-use'],
    },
    {
      type: 'category',
      label: 'Quickstart',
      collapsed: false,
      items: ['quickstart', 'first-project', 'first-gate'],
    },
    {
      type: 'category',
      label: 'Tutorials',
      collapsed: false,
      items: [
        'tutorials/tutorials',
        'tutorials/first-save-caught',
        'tutorials/rust-project',
        'tutorials/architecture',
        'tutorials/drift',
        'tutorials/policies',
      ],
    },
    {
      type: 'category',
      label: 'Core Concepts',
      collapsed: false,
      items: ['concepts/plans', 'concepts/gates', 'concepts/sessions', 'concepts/audit-trail'],
    },
    {
      type: 'category',
      label: 'Guides',
      collapsed: true,
      items: [
        'guides/solo-dev-flow',
        'guides/team-flow',
        'guides/agent-harness',
        'guides/save-time-validation',
        'guides/dashboard',
        'guides/insights',
      ],
    },
    {
      type: 'category',
      label: 'Integrations',
      collapsed: true,
      items: ['integrations/github', 'integrations/vscode', 'integrations/mcp'],
    },
    {
      type: 'category',
      label: 'Operations',
      collapsed: true,
      items: ['operations/config', 'operations/security', 'operations/troubleshooting'],
    },
    {
      type: 'category',
      label: 'Release Notes',
      collapsed: true,
      items: ['releases/changelog', 'releases/upgrade-notes'],
    },
  ],
};

export default sidebars;
