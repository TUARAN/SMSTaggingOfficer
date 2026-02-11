<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/tauri'

type ProviderHealth = {
  ok: boolean
  message: string
  model_version: string
}

type ProviderInfo = {
  kind: string
  model_path: string | null
  llama_cli_path: string | null
  ollama_base_url: string | null
  ollama_model: string | null
  temperature: number
  max_tokens: number
}

type DbStatus = {
  ok: boolean
  path: string | null
  message: string
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

type SelftestStatus = {
  running: boolean
  ok: boolean | null
  message: string
  started_at_ms: number | null
  finished_at_ms: number | null
  out_dir: string | null
}

type StatusSnapshot = {
  db: DbStatus
  provider_health: ProviderHealth
  provider: ProviderInfo
  batch: BatchProgress | null
  selftest: SelftestStatus
}

const EXPANDED_KEY = 'smsto_statusbar_expanded'

function loadExpandedPref(): boolean | null {
  try {
    const v = localStorage.getItem(EXPANDED_KEY)
    if (v === null) return null
    return v === '1'
  } catch {
    return null
  }
}

function saveExpandedPref(v: boolean) {
  try {
    localStorage.setItem(EXPANDED_KEY, v ? '1' : '0')
  } catch {
    // ignore
  }
}

const expanded = ref(false)
const autoRefresh = ref(true)
const snapshot = ref<StatusSnapshot | null>(null)
const loading = ref(false)
const error = ref<string | null>(null)

let timer: number | undefined

const modelBadgeClass = computed(() => {
  const ok = snapshot.value?.provider_health.ok
  if (ok === true) return 'good'
  if (ok === false) return 'bad'
  return ''
})

const modelBadgeText = computed(() => {
  if (!snapshot.value) return '模型：未知'
  const h = snapshot.value.provider_health
  return h.ok ? `模型：就绪（${h.model_version}）` : `模型：不可用（${h.message}）`
})

const summaryText = computed(() => {
  if (!snapshot.value) return '状态：未加载'
  const db = snapshot.value.db.ok ? 'DB✓' : 'DB×'
  const b = snapshot.value.batch
  const batch = b?.running
    ? `批处理：${b.done}/${b.total} · 模型${b.model_calls}（失败${b.model_failures}）`
    : '批处理：空闲'
  return `${db} · ${batch}`
})

function fmtTime(ms: number | null | undefined) {
  if (!ms) return '-'
  try {
    const d = new Date(ms)
    return d.toLocaleString()
  } catch {
    return String(ms)
  }
}

async function refresh() {
  loading.value = true
  error.value = null
  try {
    snapshot.value = await invoke<StatusSnapshot>('status_snapshot')
  } catch (e: any) {
    error.value = e?.toString?.() ?? String(e)
  } finally {
    loading.value = false
  }
}

onMounted(async () => {
  const pref = loadExpandedPref()
  expanded.value = pref ?? false
  await refresh()
  timer = window.setInterval(() => {
    if (autoRefresh.value && !loading.value) refresh()
  }, 5000)
})

watch(expanded, (v) => saveExpandedPref(v))

onUnmounted(() => {
  if (timer) window.clearInterval(timer)
})
</script>

<template>
  <div class="wrap">
    <div class="bar card">
      <div class="left" @click="expanded = !expanded">
        <span class="badge" :class="modelBadgeClass">{{ modelBadgeText }}</span>
        <span class="pill">{{ summaryText }}</span>
        <span v-if="snapshot" class="pill mono">
          provider={{ snapshot.provider.kind }}
        </span>
      </div>

      <div class="right">
        <label class="pill" style="cursor: pointer;">
          <input type="checkbox" v-model="autoRefresh" style="width: auto;" />
          自动刷新
        </label>
        <button @click="expanded = !expanded">{{ expanded ? '收起' : '展开' }}</button>
      </div>
    </div>

    <div v-if="expanded" class="detail card">
      <div v-if="error" class="error">{{ error }}</div>
      <div v-else-if="!snapshot" style="color: rgba(255,255,255,.65)">未加载</div>
      <div v-else class="grid">
        <div class="block">
          <div class="title">本地大模型（最重要）</div>
          <div class="row wrap" style="gap: 10px;">
            <span class="pill">health={{ snapshot.provider_health.ok ? 'ok' : 'bad' }}</span>
            <span class="pill">{{ snapshot.provider_health.message }}</span>
            <span class="pill mono">model={{ snapshot.provider_health.model_version }}</span>
          </div>
          <div class="row wrap" style="margin-top: 10px; gap: 10px;">
            <span class="pill mono">kind={{ snapshot.provider.kind }}</span>
            <span class="pill mono">temp={{ snapshot.provider.temperature }}</span>
            <span class="pill mono">max_tokens={{ snapshot.provider.max_tokens }}</span>
          </div>
          <div class="row wrap" style="margin-top: 10px; gap: 10px;">
            <span v-if="snapshot.provider.kind === 'llama_cli'" class="pill mono">model_path={{ snapshot.provider.model_path ?? '-' }}</span>
            <span v-if="snapshot.provider.kind === 'llama_cli'" class="pill mono">llama_cli={{ snapshot.provider.llama_cli_path ?? '-' }}</span>
            <span v-if="snapshot.provider.kind === 'ollama'" class="pill mono">base_url={{ snapshot.provider.ollama_base_url ?? '-' }}</span>
            <span v-if="snapshot.provider.kind === 'ollama'" class="pill mono">model={{ snapshot.provider.ollama_model ?? '-' }}</span>
          </div>
        </div>

        <div class="block">
          <div class="title">数据库</div>
          <div class="row wrap" style="gap: 10px;">
            <span class="pill">{{ snapshot.db.ok ? '可用' : '不可用' }}</span>
            <span class="pill mono">{{ snapshot.db.path ?? '-' }}</span>
            <span class="pill">{{ snapshot.db.message }}</span>
          </div>
        </div>

        <div class="block">
          <div class="title">批处理</div>
          <div class="row wrap" style="gap: 10px;">
            <span class="pill">{{ snapshot.batch?.running ? '运行中' : '空闲' }}</span>
            <span class="pill">done={{ snapshot.batch?.done ?? 0 }}/{{ snapshot.batch?.total ?? 0 }}</span>
            <span class="pill">failed={{ snapshot.batch?.failed ?? 0 }}</span>
            <span class="pill">规则强命中={{ snapshot.batch?.rule_strong_hits ?? 0 }}</span>
            <span class="pill">模型调用={{ snapshot.batch?.model_calls ?? 0 }}</span>
            <span class="pill">模型失败={{ snapshot.batch?.model_failures ?? 0 }}</span>
            <span class="pill">current={{ snapshot.batch?.current_message_id ?? '-' }}</span>
          </div>
        </div>

      </div>
    </div>
  </div>
</template>

<style scoped>
.wrap {
  position: sticky;
  bottom: 0;
  margin-top: 12px;
  padding-bottom: 12px;
  z-index: 10;
}
.bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 10px 12px;
  background: rgb(11, 16, 32);
  border: 1px solid rgba(255, 255, 255, 0.14);
  box-shadow: 0 10px 30px rgba(0, 0, 0, 0.22);
  backdrop-filter: blur(12px);
}
.left {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
  cursor: pointer;
}
.right {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
}
.detail {
  margin-top: 10px;
  background: rgb(11, 16, 32);
  border: 1px solid rgba(255, 255, 255, 0.14);
  box-shadow: 0 14px 40px rgba(0, 0, 0, 0.24);
}
.grid {
  display: grid;
  grid-template-columns: 1fr;
  gap: 12px;
}
.block .title {
  font-weight: 700;
  margin-bottom: 8px;
}
.error {
  color: rgba(255, 107, 107, 0.95);
}
</style>
