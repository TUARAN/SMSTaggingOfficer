<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/tauri'
import { listen } from '@tauri-apps/api/event'
import { useRoute, useRouter } from 'vue-router'

type BatchOptions = {
  mode: 'all' | 'unlabeled' | 'needs_review'
  concurrency: number
  timeout_ms: number
  max_retries: number
  id_min?: number
  id_max?: number
}

type BatchProgress = {
  running: boolean
  total: number
  done: number
  failed: number
  rule_strong_hits: number
  model_calls: number
  model_failures: number
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
  rule_strong_hits: 0,
  model_calls: 0,
  model_failures: 0,
  current_message_id: null,
  started_at_ms: null,
  elapsed_ms: 0
})

const router = useRouter()
const route = useRoute()
const showCompleted = ref(false)
const completedSummary = ref('')

const busy = computed(() => progress.value.running)
const pct = computed(() => {
  if (!progress.value.total) return 0
  return Math.floor((progress.value.done / progress.value.total) * 100)
})

async function refreshStatus() {
  progress.value = await invoke<BatchProgress>('batch_status')
}

async function start() {
  showCompleted.value = false
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
  const q = route.query
  const mode = (Array.isArray(q.mode) ? q.mode[0] : q.mode) as any
  if (mode === 'all' || mode === 'unlabeled' || mode === 'needs_review') {
    options.value.mode = mode
  }
  const idMinRaw = Array.isArray(q.id_min) ? q.id_min[0] : q.id_min
  const idMaxRaw = Array.isArray(q.id_max) ? q.id_max[0] : q.id_max
  const idMin = idMinRaw != null ? Number(idMinRaw) : NaN
  const idMax = idMaxRaw != null ? Number(idMaxRaw) : NaN
  if (Number.isFinite(idMin)) options.value.id_min = idMin
  if (Number.isFinite(idMax)) options.value.id_max = idMax

  await refreshStatus()
  await listen<BatchProgress>('batch_progress', (e) => {
    progress.value = e.payload
  })
})

watch(
  () => progress.value.running,
  (running, prev) => {
    if (prev === true && running === false && progress.value.total > 0 && progress.value.done >= progress.value.total) {
      const p = progress.value
      completedSummary.value = `完成：${p.done}/${p.total} · 失败${p.failed} · 规则强命中${p.rule_strong_hits} · 模型调用${p.model_calls}（失败${p.model_failures}）`
      showCompleted.value = true
    }
  }
)

function gotoList() {
  router.push({ path: '/list' })
}

function gotoReview() {
  router.push({ path: '/list', query: { needs_review: '1' } })
}
</script>

<template>
  <div class="page">
    <div v-if="showCompleted" class="card" style="border: 1px solid rgba(56, 211, 159, 0.35);">
      <div class="row wrap" style="justify-content: space-between; align-items: center; gap: 10px;">
        <div>
          <div style="font-weight: 800;">批处理完成</div>
          <div style="color: rgba(255,255,255,.75); margin-top: 4px;">{{ completedSummary }}</div>
        </div>
        <div class="row wrap" style="gap: 10px;">
          <button @click="gotoList">去列表</button>
          <button class="primary" @click="gotoReview">去复核（needs_review）</button>
        </div>
      </div>
    </div>

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
          <span class="pill">规则强命中：{{ progress.rule_strong_hits }}</span>
          <span class="pill">模型调用：{{ progress.model_calls }}</span>
          <span class="pill">模型失败：{{ progress.model_failures }}</span>
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
