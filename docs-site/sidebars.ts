import type {SidebarsConfig} from '@docusaurus/plugin-content-docs';

const sidebars: SidebarsConfig = {
  docsSidebar: [
    'intro',
    {
      type: 'category',
      label: 'Getting Started',
      items: [
        'getting-started/installation',
        'getting-started/quick-start',
      ],
    },
    {
      type: 'category',
      label: 'Development',
      items: [
        'development/building',
        'development/testing',
        'development/ci',
      ],
    },
    {
      type: 'category',
      label: 'Reference',
      items: [
        {
          type: 'link',
          label: 'Architecture (Spec)',
          href: 'https://github.com/koedame/p2paudiosession/blob/main/docs-spec/architecture.md',
        },
        {
          type: 'link',
          label: 'API Specifications',
          href: 'https://github.com/koedame/p2paudiosession/tree/main/docs-spec/api',
        },
        {
          type: 'link',
          label: 'ADR (Design Decisions)',
          href: 'https://github.com/koedame/p2paudiosession/tree/main/docs-spec/adr',
        },
      ],
    },
  ],
};

export default sidebars;
