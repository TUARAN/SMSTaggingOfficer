<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/tauri'
import { listen } from '@tauri-apps/api/event'

type BatchOptions = {
  mode: 'all' | 'unlabeled' | 'needs_review'
  concurrency: number
  timeout_ms: number
  max_retries: number
}

type BatchProgress = {
  running: boolean
  total: number
  done: number
  failed: number
  current_message_id: number | null
  started_at_ms: number | null
  elapsed_ms: number
}

const options = ref<BatchOptions>({
  mode: 'unlabeled',
  concurrency: 2,
  timeout_ms: 15000,
  max_retries: 1
})

const progress = ref<BatchProgress>({
  running: false,
  total: 0,
  done: 0,
  failed: 0,
  current_message_id: null,
  started_at_ms: null,
  elapsed_ms: 0
})

const busy = computed(() => progress.value.running)
const pct = computed(() => {
  if (!progress.value.total) return 0
  return Math.floor((progress.value.done / progress.value.total) * 100)
})

async function refreshStatus() {
  progress.value = await invoke<BatchProgress>('batch_status')
}

async function start() {
  await invoke('batch_start', { options: options.value })
  await refreshStatus()
}

async function stop() {
  await invoke('batch_stop')
  await refreshStatus()
}

async function retryFailed() {
  await invoke('batch_retry_failed')
  await refreshStatus()
}

onMounted(async () => {
  await refreshStatus()
  await listen<BatchProgress>('batch_progress', (e) => {
    progress.value = e.payload
  })
})
</script>

<template>
  <div class="page">
    <div class="row wrap" style="justify-content: space-between; align-items: flex-end;">
      <div>
        <h2 style="margin: 0;">批处理</h2>
        <div style="color: rgba(255,255,255,.65); margin-top: 6px;">队列化推理，后台并发执行，UI 不会卡死；支持超时、重试与失败计数。</div>
      </div>
      <div class="row">
        <button class="primary" :disabled="busy" @click="start">开始</button>
        <button class="danger" :disabled="!busy" @click="stop">停止</button>
        <button :disabled="busy || progress.failed === 0" @click="retryFailed">重试失败</button>
      </div>
    </div>

    <div class="sep" />

    <div class="card">
      <div class="grid">
        <div class="kv">
          <div style="color: rgba(255,255,255,.65)">增量模式</div>
          <select v-model="options.mode" :disabled="busy">
            <option value="all">全量</option>
            <option value="unlabeled">只跑未标注</option>
            <option value="needs_review">只跑 needs_review</option>
          </select>
        </div>
        <div class="kv">
          <div style="color: rgba(255,255,255,.65)">并发</div>
          <input type="number" min="1" max="8" v-model.number="options.concurrency" :disabled="busy" />
        </div>
        <div class="kv">
          <div style="color: rgba(255,255,255,.65)">超时(ms)</div>
          <input type="number" min="3000" step="1000" v-model.number="options.timeout_ms" :disabled="busy" />
        </div>
        <div class="kv">
          <div style="color: rgba(255,255,255,.65)">最大重试</div>
          <input type="number" min="0" max="3" v-model.number="options.max_retries" :disabled="busy" />
        </div>
      </div>

      <div class="sep" />

      <div class="row wrap" style="justify-content: space-between;">
        <div class="row wrap" style="gap: 10px;">
          <span class="pill">状态：{{ progress.running ? '运行中' : '空闲' }}</span>
          <span class="pill">进度：{{ progress.done }}/{{ progress.total }}（{{ pct }}%）</span>
          <span class="pill">失败：{{ progress.failed }}</span>
          <span class="pill">当前：{{ progress.current_message_id ?? '-' }}</span>
          <span class="pill">耗时：{{ Math.floor(progress.elapsed_ms / 1000) }}s</span>
        </div>
        <button @click="refreshStatus">刷新</button>
      </div>

      <div style="margin-top: 12px;">
        <div class="bar">
          <div class="fill" :style="{ width: pct + '%' }" />
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.page { display: flex; flex-direction: column; gap: 12px; }
.grid { display: grid; grid-template-columns: 1fr 1fr; gap: 10px 14px; }
@media (max-width: 980px) {
  .grid { grid-template-columns: 1fr; }
}
.bar {
  height: 12px;
  border-radius: 999px;
  background: rgba(255, 255, 255, 0.07);
  border: 1px solid rgba(255, 255, 255, 0.10);
  overflow: hidden;
}
.fill {
  height: 100%;
  background: linear-gradient(90deg, rgba(124, 92, 255, 0.85), rgba(56, 211, 159, 0.65));
}
</style>
