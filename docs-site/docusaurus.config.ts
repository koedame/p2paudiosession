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
          href: 'https://github.com/koedame/p2paudiosession/tree/main/docs-spec',
          label: 'Spec',
          position: 'left',
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
              href: 'https://github.com/koedame/p2paudiosession/blob/main/docs-spec/architecture.md',
            },
            {
              label: 'API Specifications',
              href: 'https://github.com/koedame/p2paudiosession/tree/main/docs-spec/api',
            },
            {
              label: 'ADR (Design Decisions)',
              href: 'https://github.com/koedame/p2paudiosession/tree/main/docs-spec/adr',
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
