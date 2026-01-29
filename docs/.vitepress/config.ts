import { defineConfig } from 'vitepress'

export default defineConfig({
  title: 'Better GraphQL',
  description: 'A modern, type-safe GraphQL implementation with Rust-inspired features',

  // Ignore dead links during development - pages are still being created
  ignoreDeadLinks: true,

  head: [
    ['link', { rel: 'icon', type: 'image/svg+xml', href: '/logo.svg' }],
  ],

  themeConfig: {
    logo: '/logo.svg',

    nav: [
      { text: 'Guide', link: '/guide/getting-started' },
      { text: 'Schema', link: '/schema/types' },
      { text: 'Backend', link: '/backend/quickstart' },
      { text: 'Frontend', link: '/frontend/quickstart' },
      { text: 'CLI', link: '/cli/overview' },
      { text: 'API', link: '/api/overview' },
    ],

    sidebar: {
      '/guide/': [
        {
          text: 'Introduction',
          items: [
            { text: 'Getting Started', link: '/guide/getting-started' },
            { text: 'Why Better GraphQL?', link: '/guide/why-bgql' },
            { text: 'Installation', link: '/guide/installation' },
          ]
        },
        {
          text: 'Core Concepts',
          items: [
            { text: 'Type System', link: '/guide/type-system' },
            { text: 'Module System', link: '/guide/modules' },
            { text: 'Error Handling', link: '/guide/error-handling' },
          ]
        }
      ],
      '/schema/': [
        {
          text: 'Schema Definition',
          items: [
            { text: 'Types', link: '/schema/types' },
            { text: 'Interfaces', link: '/schema/interfaces' },
            { text: 'Enums & Unions', link: '/schema/enums-unions' },
            { text: 'Input Types', link: '/schema/inputs' },
            { text: 'Generics', link: '/schema/generics' },
            { text: 'Directives', link: '/schema/directives' },
          ]
        },
        {
          text: 'Advanced',
          items: [
            { text: 'Module System', link: '/schema/modules' },
            { text: 'Opaque Types', link: '/schema/opaque-types' },
            { text: 'Streaming', link: '/schema/streaming' },
          ]
        }
      ],
      '/backend/': [
        {
          text: 'Server SDK',
          items: [
            { text: 'Quick Start', link: '/backend/quickstart' },
            { text: 'Resolvers', link: '/backend/resolvers' },
            { text: 'Context', link: '/backend/context' },
            { text: 'Authentication', link: '/backend/authentication' },
            { text: 'Error Handling', link: '/backend/errors' },
          ]
        },
        {
          text: 'Advanced',
          items: [
            { text: 'DataLoader', link: '/backend/dataloader' },
            { text: 'Subscriptions', link: '/backend/subscriptions' },
            { text: 'Streaming', link: '/backend/streaming' },
            { text: 'Testing', link: '/backend/testing' },
          ]
        },
        {
          text: 'Deployment',
          items: [
            { text: 'Production', link: '/backend/production' },
            { text: 'Performance', link: '/backend/performance' },
          ]
        }
      ],
      '/frontend/': [
        {
          text: 'Client SDK',
          items: [
            { text: 'Quick Start', link: '/frontend/quickstart' },
            { text: 'Queries', link: '/frontend/queries' },
            { text: 'Mutations', link: '/frontend/mutations' },
            { text: 'Type Safety', link: '/frontend/type-safety' },
          ]
        },
        {
          text: 'Framework Integration',
          items: [
            { text: 'Vue.js', link: '/frontend/vue' },
            { text: 'React', link: '/frontend/react' },
            { text: 'Svelte', link: '/frontend/svelte' },
          ]
        },
        {
          text: 'Advanced',
          items: [
            { text: 'Caching', link: '/frontend/caching' },
            { text: 'Subscriptions', link: '/frontend/subscriptions' },
            { text: 'Streaming', link: '/frontend/streaming' },
            { text: 'Error Handling', link: '/frontend/errors' },
          ]
        }
      ],
      '/cli/': [
        {
          text: 'CLI',
          items: [
            { text: 'Overview', link: '/cli/overview' },
            { text: 'Commands', link: '/cli/commands' },
            { text: 'Code Generation', link: '/cli/codegen' },
          ]
        }
      ],
      '/api/': [
        {
          text: 'API Reference',
          items: [
            { text: 'Overview', link: '/api/overview' },
            { text: 'Server API', link: '/api/server' },
            { text: 'Client API', link: '/api/client' },
          ]
        }
      ]
    },

    socialLinks: [
      { icon: 'github', link: 'https://github.com/ubugeeei/bgql' }
    ],

    footer: {
      message: 'Released under the MIT License.',
      copyright: 'Copyright © 2024 ubugeeei'
    },

    search: {
      provider: 'local'
    },

    editLink: {
      pattern: 'https://github.com/ubugeeei/bgql/edit/main/docs/:path',
      text: 'Edit this page on GitHub'
    }
  },

  markdown: {
    theme: {
      light: 'github-light',
      dark: 'github-dark'
    },
    lineNumbers: true
  }
})
