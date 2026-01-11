import {themes as prismThemes} from 'prism-react-renderer';
import type {Config} from '@docusaurus/types';
import type * as Preset from '@docusaurus/preset-classic';

const config: Config = {
  title: 'jamjam Documentation',
  tagline: 'P2P Audio Communication for Musicians',
  favicon: 'img/favicon.ico',

  future: {
    v4: true,
  },

  markdown: {
    mermaid: true,
  },
  themes: ['@docusaurus/theme-mermaid'],

  url: 'https://koedame.github.io',
  baseUrl: '/p2paudiosession/',

  organizationName: 'koedame',
  projectName: 'p2paudiosession',

  onBrokenLinks: 'throw',

  i18n: {
    defaultLocale: 'ja',
    locales: ['ja', 'en'],
  },

  presets: [
    [
      'classic',
      {
        docs: {
          sidebarPath: './sidebars.ts',
          editUrl:
            'https://github.com/koedame/p2paudiosession/edit/main/docs-site/',
        },
        blog: false,
        theme: {
          customCss: './src/css/custom.css',
        },
      } satisfies Preset.Options,
    ],
  ],

  plugins: [
    [
      '@docusaurus/plugin-content-docs',
      {
        id: 'spec',
        path: '../docs-spec',
        routeBasePath: 'spec',
        sidebarPath: './sidebarsSpec.ts',
        editUrl:
          'https://github.com/koedame/p2paudiosession/edit/main/',
      },
    ],
  ],

  themeConfig: {
    colorMode: {
      respectPrefersColorScheme: true,
    },
    navbar: {
      title: 'jamjam',
      items: [
        {
          type: 'docSidebar',
          sidebarId: 'docsSidebar',
          position: 'left',
          label: 'Docs',
        },
        {
          type: 'docSidebar',
          sidebarId: 'specSidebar',
          docsPluginId: 'spec',
          position: 'left',
          label: 'Spec',
        },
        {
          href: 'https://github.com/koedame/p2paudiosession',
          label: 'GitHub',
          position: 'right',
        },
      ],
    },
    footer: {
      style: 'dark',
      links: [
        {
          title: 'Docs',
          items: [
            {
              label: 'Getting Started',
              to: '/docs/intro',
            },
            {
              label: 'Development',
              to: '/docs/development/building',
            },
          ],
        },
        {
          title: 'Specification',
          items: [
            {
              label: 'Architecture',
              to: '/spec/architecture',
            },
            {
              label: 'API Specifications',
              to: '/spec/api/audio_engine',
            },
            {
              label: 'ADR (Design Decisions)',
              to: '/spec/adr/ADR-001-language-rust',
            },
          ],
        },
        {
          title: 'More',
          items: [
            {
              label: 'GitHub',
              href: 'https://github.com/koedame/p2paudiosession',
            },
          ],
        },
      ],
      copyright: `Copyright © ${new Date().getFullYear()} koedame. Built with Docusaurus.`,
    },
    prism: {
      theme: prismThemes.github,
      darkTheme: prismThemes.dracula,
      additionalLanguages: ['rust', 'toml', 'bash'],
    },
  } satisfies Preset.ThemeConfig,
};

export default config;
