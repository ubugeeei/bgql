<script setup lang="ts">
import { ref } from 'vue'

const showMenu = ref(false)

const examples = [
  { name: 'Basic Schema', icon: 'mdi-file-document-outline' },
  { name: 'E-commerce API', icon: 'mdi-cart-outline' },
  { name: 'Social Network', icon: 'mdi-account-group-outline' },
  { name: 'Blog Platform', icon: 'mdi-post-outline' },
]
</script>

<template>
  <header class="header">
    <div class="header__brand">
      <svg class="logo" viewBox="0 0 100 100" fill="none" xmlns="http://www.w3.org/2000/svg">
        <defs>
          <linearGradient id="bgql-grad" x1="0%" y1="0%" x2="100%" y2="100%">
            <stop offset="0%" stop-color="#58a6ff"/>
            <stop offset="50%" stop-color="#a371f7"/>
            <stop offset="100%" stop-color="#f778ba"/>
          </linearGradient>
        </defs>
        <circle cx="50" cy="50" r="46" fill="#0d1117" stroke="url(#bgql-grad)" stroke-width="2"/>
        <polygon points="50,12 82,30 82,70 50,88 18,70 18,30" fill="none" stroke="url(#bgql-grad)" stroke-width="2.5" stroke-linejoin="round"/>
        <g stroke="url(#bgql-grad)" stroke-width="1.5" opacity="0.5">
          <line x1="50" y1="12" x2="50" y2="88"/>
          <line x1="18" y1="30" x2="82" y2="70"/>
          <line x1="82" y1="30" x2="18" y2="70"/>
        </g>
        <g fill="url(#bgql-grad)">
          <circle cx="50" cy="12" r="4"/>
          <circle cx="82" cy="30" r="4"/>
          <circle cx="82" cy="70" r="4"/>
          <circle cx="50" cy="88" r="4"/>
          <circle cx="18" cy="70" r="4"/>
          <circle cx="18" cy="30" r="4"/>
        </g>
        <text x="50" y="58" text-anchor="middle" fill="url(#bgql-grad)" font-family="system-ui" font-weight="700" font-size="28">B</text>
      </svg>
      <span class="title">bgql</span>
      <span class="badge">Playground</span>
    </div>

    <div class="header__actions">
      <div class="dropdown">
        <button class="btn btn-secondary" @click="showMenu = !showMenu">
          <i class="mdi mdi-folder-open-outline"></i>
          Examples
          <i class="mdi mdi-chevron-down"></i>
        </button>

        <div v-if="showMenu" class="dropdown__menu">
          <div
            v-for="example in examples"
            :key="example.name"
            class="dropdown__item"
            @click="showMenu = false"
          >
            <i :class="['mdi', example.icon]"></i>
            {{ example.name }}
          </div>
        </div>
      </div>

      <button class="btn btn-secondary">
        <i class="mdi mdi-cog-outline"></i>
        Settings
      </button>

      <a
        href="https://github.com/ubugeeei/better-graphql"
        target="_blank"
        class="btn btn-ghost btn-icon"
        title="View on GitHub"
      >
        <i class="mdi mdi-github"></i>
      </a>
    </div>
  </header>
</template>
