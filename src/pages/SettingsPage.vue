<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { open } from '@tauri-apps/api/dialog'
import { invoke } from '@tauri-apps/api/tauri'

type AppSettings = {
  provider: {
    kind: 'llama_cli' | 'ollama' | 'mock'
    model_path: string | null
    llama_cli_path: string | null
    ollama_base_url?: string | null
    ollama_model?: string | null
    temperature: number
    max_tokens: number
  }
}

type Health = {
  ok: boolean
  message: string
  model_version: string
}

const settings = ref<AppSettings | null>(null)
const health = ref<Health | null>(null)
const saving = ref(false)

async function load() {
  settings.value = await invoke<AppSettings>('settings_get')
}

async function pickModel() {
  const selected = await open({
    title: '选择 GGUF 模型文件',
    multiple: false,
    filters: [{ name: 'GGUF', extensions: ['gguf'] }]
  })
  if (typeof selected === 'string' && settings.value) {
    settings.value.provider.model_path = selected
  }
}

async function pickLlamaCli() {
  const selected = await open({
    title: '选择 llama-cli 可执行文件（可选：打包内置或手动指定）',
    multiple: false
  })
  if (typeof selected === 'string' && settings.value) {
    settings.value.provider.llama_cli_path = selected
  }
}

async function saveSettings() {
  if (!settings.value) return
  saving.value = true
  try {
    await invoke('settings_set', { settings: settings.value })
  } finally {
    saving.value = false
  }
}

async function check() {
  health.value = await invoke<Health>('provider_health_check')
}

onMounted(load)
</script>

<template>
  <div class="page">
    <div class="row wrap" style="justify-content: space-between; align-items: flex-end;">
      <div>
        <h2 style="margin: 0;">设置</h2>
        <div style="color: rgba(255,255,255,.65); margin-top: 6px;">选择本地 GGUF 模型路径并做健康检查（完全离线）。</div>
      </div>
      <div class="row">
        <button class="primary" :disabled="saving || !settings" @click="saveSettings">保存</button>
        <button :disabled="saving || !settings" @click="check">健康检查</button>
      </div>
    </div>

    <div class="sep" />

    <div class="card" v-if="settings">
      <div class="grid">
        <div class="kv">
          <div style="color: rgba(255,255,255,.65)">Provider</div>
          <select v-model="settings.provider.kind">
            <option value="llama_cli">llama.cpp（llama-cli）</option>
            <option value="ollama">Ollama（本机服务）</option>
            <option value="mock">Mock（仅规则/演示）</option>
          </select>
        </div>

        <template v-if="settings.provider.kind === 'llama_cli'">
          <div class="kv" style="grid-column: 1 / -1;">
            <div style="color: rgba(255,255,255,.65)">模型文件</div>
            <div class="row" style="gap: 10px;">
              <input v-model="settings.provider.model_path" placeholder="选择 *.gguf" />
              <button @click="pickModel">选择</button>
            </div>
          </div>

          <div class="kv" style="grid-column: 1 / -1;">
            <div style="color: rgba(255,255,255,.65)">llama-cli 路径（可选）</div>
            <div class="row" style="gap: 10px;">
              <input v-model="settings.provider.llama_cli_path" placeholder="留空则使用打包内置（resources）" />
              <button @click="pickLlamaCli">选择</button>
            </div>
          </div>
        </template>

        <template v-else-if="settings.provider.kind === 'ollama'">
          <div class="kv" style="grid-column: 1 / -1;">
            <div style="color: rgba(255,255,255,.65)">Ollama 地址</div>
            <input v-model="settings.provider.ollama_base_url" placeholder="http://127.0.0.1:11434" />
          </div>

          <div class="kv" style="grid-column: 1 / -1;">
            <div style="color: rgba(255,255,255,.65)">Ollama 模型名</div>
            <input v-model="settings.provider.ollama_model" placeholder="llama3.2:1b" />
          </div>

          <div class="pill" style="grid-column: 1 / -1;">
            提示：先执行 <span class="mono">ollama pull llama3.2:1b</span>，并确保 Ollama 正在运行。
          </div>
        </template>

        <div class="kv">
          <div style="color: rgba(255,255,255,.65)">temperature</div>
          <input type="number" step="0.1" min="0" max="1" v-model.number="settings.provider.temperature" />
        </div>
        <div class="kv">
          <div style="color: rgba(255,255,255,.65)">max_tokens</div>
          <input type="number" min="64" step="64" v-model.number="settings.provider.max_tokens" />
        </div>
      </div>

      <div class="sep" />

      <div v-if="health" class="pill">
        {{ health.ok ? 'OK' : 'FAILED' }} · {{ health.message }} · model_version={{ health.model_version }}
      </div>
    </div>

    <div class="card" v-else>
      <div style="color: rgba(255,255,255,.65)">加载设置中…</div>
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
