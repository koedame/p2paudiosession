import type {SidebarsConfig} from '@docusaurus/plugin-content-docs';

const sidebars: SidebarsConfig = {
  specSidebar: [
    'README',
    'architecture',
    {
      type: 'category',
      label: 'API Specifications',
      items: [
        'api/audio_engine',
        'api/network',
        'api/signaling',
        'api/plugin',
        'api/i18n',
      ],
    },
    {
      type: 'category',
      label: 'ADR (Design Decisions)',
      items: [
        'adr/ADR-001-language-rust',
        'adr/ADR-002-network-protocol',
        'adr/ADR-003-audio-codec',
        'adr/ADR-004-gui-framework',
        'adr/ADR-005-no-audio-processing',
        'adr/ADR-006-fec-strategy',
        'adr/ADR-007-i18n-library',
      ],
    },
  ],
};

export default sidebars;
