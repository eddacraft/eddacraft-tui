import type { SidebarsConfig } from '@docusaurus/plugin-content-docs';

const sidebars: SidebarsConfig = {
  anvilSidebar: [
    {
      type: 'category',
      label: 'Start here',
      collapsed: false,
      items: ['overview', 'when-to-use', 'quickstart', 'first-gate', 'concepts/glossary'],
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
      label: 'How-to guides',
      collapsed: true,
      items: [
        'first-project',
        'guides/solo-dev-flow',
        'guides/team-flow',
        'guides/agent-harness',
        'guides/save-time-validation',
        'guides/dashboard',
        'guides/insights',
        'integrations/github',
        'integrations/mcp',
        'integrations/agent-skills',
        'integrations/vscode',
        'operations/git-hooks',
        'operations/troubleshooting',
        'operations/uninstall',
      ],
    },
    {
      type: 'category',
      label: 'Concepts',
      collapsed: true,
      items: [
        'concepts/gates',
        'concepts/plans',
        'concepts/sessions',
        'concepts/audit-trail',
        'concepts/review-capsules',
      ],
    },
    {
      type: 'category',
      label: 'Reference',
      collapsed: true,
      items: [
        'reference/cli-reference',
        'reference/rule-reference',
        'reference/support-reference',
        'operations/config',
        'operations/security',
        'operations/telemetry',
        'integrations/watch-output',
        'guides/start-output-contracts',
      ],
    },
    {
      type: 'category',
      label: 'Beta',
      collapsed: true,
      items: ['beta-testing-guide'],
    },
    {
      type: 'category',
      label: 'Releases',
      collapsed: true,
      items: ['releases/upgrade-notes', 'releases/changelog', 'releases/rust-rewrite'],
    },
  ],
};

export default sidebars;
