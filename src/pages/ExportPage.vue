<script setup lang="ts">
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/tauri'
import { save } from '@tauri-apps/api/dialog'

type ExportFormat = 'csv' | 'jsonl'

type ExportOptions = {
  only_reviewed: boolean
  format: ExportFormat
}

const options = ref<ExportOptions>({
  only_reviewed: false,
  format: 'jsonl'
})

const exporting = ref(false)
const result = ref('')

async function doExport() {
  const ext = options.value.format === 'csv' ? 'csv' : 'jsonl'
  const path = await save({
    title: '选择导出文件位置',
    defaultPath: `sms_export.${ext}`
  })
  if (!path) return

  exporting.value = true
  result.value = ''
  try {
    const written = await invoke<number>('export_execute', {
      path,
      options: options.value
    })
    result.value = `导出完成：写入 ${written} 行 -> ${path}`
  } catch (e: any) {
    result.value = `导出失败：${e?.message ?? String(e)}`
  } finally {
    exporting.value = false
  }
}
</script>

<template>
  <div class="page">
    <div class="row wrap" style="justify-content: space-between; align-items: flex-end;">
      <div>
        <h2 style="margin: 0;">导出</h2>
        <div style="color: rgba(255,255,255,.65); margin-top: 6px;">支持 CSV 与 JSONL；可选择只导出已复核样本（needs_review=false）或全量。</div>
      </div>
      <div class="row">
        <button class="primary" :disabled="exporting" @click="doExport">导出</button>
      </div>
    </div>

    <div class="sep" />

    <div class="card">
      <div class="grid">
        <div class="kv">
          <div style="color: rgba(255,255,255,.65)">格式</div>
          <select v-model="options.format" :disabled="exporting">
            <option value="jsonl">JSONL（推荐）</option>
            <option value="csv">CSV</option>
          </select>
        </div>
        <div class="kv">
          <div style="color: rgba(255,255,255,.65)">只导出已复核</div>
          <select v-model="options.only_reviewed" :disabled="exporting">
            <option :value="false">否（全量）</option>
            <option :value="true">是（needs_review=false）</option>
          </select>
        </div>
      </div>

      <div class="sep" />
      <div v-if="result" class="pill">{{ result }}</div>
    </div>
  </div>
</template>

<style scoped>
.page { display: flex; flex-direction: column; gap: 12px; }
.grid { display: grid; grid-template-columns: 1fr 1fr; gap: 10px 14px; }
@media (max-width: 980px) {
  .grid { grid-template-columns: 1fr; }
}
</style>
