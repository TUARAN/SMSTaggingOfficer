import { createRouter, createWebHashHistory } from 'vue-router'

import ImportPage from './pages/ImportPage.vue'
import BatchPage from './pages/BatchPage.vue'
import ListPage from './pages/ListPage.vue'
import ExportPage from './pages/ExportPage.vue'
import SettingsPage from './pages/SettingsPage.vue'

export const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: '/', redirect: '/import' },
    { path: '/import', component: ImportPage },
    { path: '/batch', component: BatchPage },
    { path: '/list', component: ListPage },
    { path: '/export', component: ExportPage },
    { path: '/settings', component: SettingsPage }
  ]
})
