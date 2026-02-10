<script setup lang="ts">
import { computed } from 'vue'
import { useRoute } from 'vue-router'

const route = useRoute()
const active = computed(() => route.path)

const nav = [
  { path: '/import', label: '导入' },
  { path: '/batch', label: '批处理' },
  { path: '/list', label: '列表/复核' },
  { path: '/export', label: '导出' },
  { path: '/settings', label: '设置' }
]
</script>

<template>
  <div class="layout">
    <aside class="sidebar">
      <div class="brand">
        <div class="title">短信智标官</div>
        <div class="sub">SMS Tagging Officer</div>
      </div>
      <nav class="nav">
        <RouterLink
          v-for="item in nav"
          :key="item.path"
          class="navItem"
          :class="{ active: active === item.path }"
          :to="item.path"
        >
          {{ item.label }}
        </RouterLink>
      </nav>
      <div class="footer">
        <span class="pill">完全离线 · SQLite · llama.cpp</span>
      </div>
    </aside>

    <main class="main">
      <RouterView />
    </main>
  </div>
</template>

<style scoped>
.layout {
  display: grid;
  grid-template-columns: 260px 1fr;
  height: 100vh;
}
.sidebar {
  border-right: 1px solid rgba(255, 255, 255, 0.10);
  background: rgba(0, 0, 0, 0.2);
  backdrop-filter: blur(10px);
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 16px;
}
.brand .title { font-weight: 800; font-size: 18px; letter-spacing: 0.5px; }
.brand .sub { font-size: 12px; color: rgba(255, 255, 255, 0.65); margin-top: 2px; }

.nav { display: flex; flex-direction: column; gap: 10px; }
.navItem {
  border: 1px solid rgba(255, 255, 255, 0.10);
  border-radius: 12px;
  padding: 10px 12px;
  background: rgba(255, 255, 255, 0.04);
}
.navItem.active {
  border-color: rgba(124, 92, 255, 0.55);
  background: rgba(124, 92, 255, 0.12);
}

.main {
  padding: 18px;
  overflow: auto;
}
.footer { margin-top: auto; }
</style>
