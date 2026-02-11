<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { open } from '@tauri-apps/api/dialog'
import { invoke } from '@tauri-apps/api/tauri'
import { useRouter } from 'vue-router'

type ImportPreview = {
  headers: string[]
  rows: Record<string, string>[]
}

type ColumnMapping = {
  content: string
  received_at?: string
  sender?: string
  phone?: string
  source?: string
}

type ImportExecuteResult = {
  total_rows: number
  valid_rows: number
  inserted: number
  skipped_empty_content: number
  first_insert_id: number | null
  last_insert_id: number | null
}

type DbMeta = {
  messages_count: number
  messages_max_id: number
}

const filePath = ref<string | null>(null)
const preview = ref<ImportPreview | null>(null)
const mapping = ref<ColumnMapping>({ content: '' })
const importing = ref(false)
const importResult = ref<string>('')
const execResult = ref<ImportExecuteResult | null>(null)
const dbMeta = ref<DbMeta | null>(null)
const dbMetaAfter = ref<DbMeta | null>(null)

const router = useRouter()

async function refreshDbMeta() {
  dbMeta.value = await invoke<DbMeta>('messages_meta')
}

async function pickFile() {
  const selected = await open({
    title: '选择 CSV / Excel 文件',
    multiple: false,
    filters: [
      { name: 'Data', extensions: ['csv', 'xlsx'] }
    ]
  })
  if (typeof selected === 'string') {
    filePath.value = selected
    importResult.value = ''
    execResult.value = null
    dbMetaAfter.value = null
    await refreshDbMeta()
    preview.value = await invoke<ImportPreview>('import_preview', { path: selected })
    const headers = preview.value.headers

    const pickFirst = (candidates: string[]) => candidates.find((h) => headers.includes(h))

    mapping.value.content =
      pickFirst(['content', '短信内容', '内容', 'message', 'text']) ?? (headers[0] ?? '')
    mapping.value.received_at = pickFirst(['received_at', '时间', '日期', 'receivedAt'])
    mapping.value.sender = pickFirst(['sender', '发送方', '机构', '品牌', 'brand'])
    mapping.value.phone = pickFirst(['phone', '手机号', '电话'])
    mapping.value.source = pickFirst(['source', '来源'])

    // UX: selecting a file means "import it" (no extra button click).
    await doImport()
  }
}

async function doImport() {
  if (!filePath.value) return
  if (!mapping.value.content) {
    importResult.value = '必须映射 content 列'
    return
  }
  importing.value = true
  importResult.value = ''
  try {
    const res = await invoke<ImportExecuteResult>('import_execute', {
      path: filePath.value,
      mapping: mapping.value
    })
    execResult.value = res
    dbMetaAfter.value = await invoke<DbMeta>('messages_meta')
    importResult.value = `导入完成：解析 ${res.total_rows} 行（有效 ${res.valid_rows}，跳过空内容 ${res.skipped_empty_content}），入库新增 ${res.inserted} 条短信`
  } catch (e: any) {
    importResult.value = `导入失败：${e?.message ?? String(e)}`
  } finally {
    importing.value = false
  }
}

onMounted(refreshDbMeta)

function gotoBatchThisImport() {
  const r = execResult.value
  if (!r?.inserted) {
    router.push({ path: '/batch' })
    return
  }
  router.push({
    path: '/batch',
    query: {
      mode: 'all',
      id_min: String(r.first_insert_id ?? ''),
      id_max: String(r.last_insert_id ?? '')
    }
  })
}

function gotoBatchUnlabeled() {
  router.push({ path: '/batch', query: { mode: 'unlabeled' } })
}
</script>

<template>
  <div class="page">
    <div class="row wrap" style="justify-content: space-between; align-items: flex-end;">
      <div>
        <h2 style="margin: 0;">导入</h2>
        <div style="color: rgba(255,255,255,.65); margin-top: 6px;">支持 CSV / Excel（.xlsx），可做列映射，离线导入到 SQLite。</div>
      </div>
      <div class="row">
        <button class="primary" :disabled="importing" @click="pickFile">{{ filePath ? '重新选择' : '选择文件' }}</button>
      </div>
    </div>

    <div class="sep" />

    <div class="card" v-if="filePath">
      <div class="row wrap" style="justify-content: space-between;">
        <div>
          <div style="font-weight: 700;">已选择文件</div>
          <div class="mono" style="color: rgba(255,255,255,.65); font-size: 12px; margin-top: 4px;">{{ filePath }}</div>
          <div v-if="dbMeta" style="color: rgba(255,255,255,.65); font-size: 12px; margin-top: 6px;">
            当前数据库：{{ dbMeta.messages_count }} 条 · 最大 ID：{{ dbMeta.messages_max_id }}（ID 是主键，不保证连续）
          </div>
          <div v-if="dbMetaAfter" style="color: rgba(255,255,255,.65); font-size: 12px; margin-top: 4px;">
            导入后：{{ dbMetaAfter.messages_count }} 条 · 最大 ID：{{ dbMetaAfter.messages_max_id }}
          </div>
        </div>
      </div>

      <div class="sep" />

      <div style="font-weight: 700; margin-bottom: 8px;">列映射</div>
      <div class="grid">
        <div class="kv">
          <div style="color: rgba(255,255,255,.65)">content *</div>
          <select v-model="mapping.content">
            <option value="">请选择</option>
            <option v-for="h in preview?.headers ?? []" :key="h" :value="h">{{ h }}</option>
          </select>
        </div>
        <div class="kv">
          <div style="color: rgba(255,255,255,.65)">received_at</div>
          <select v-model="mapping.received_at">
            <option :value="undefined">(不导入)</option>
            <option v-for="h in preview?.headers ?? []" :key="h" :value="h">{{ h }}</option>
          </select>
        </div>
        <div class="kv">
          <div style="color: rgba(255,255,255,.65)">sender</div>
          <select v-model="mapping.sender">
            <option :value="undefined">(不导入)</option>
            <option v-for="h in preview?.headers ?? []" :key="h" :value="h">{{ h }}</option>
          </select>
        </div>
        <div class="kv">
          <div style="color: rgba(255,255,255,.65)">phone</div>
          <select v-model="mapping.phone">
            <option :value="undefined">(不导入)</option>
            <option v-for="h in preview?.headers ?? []" :key="h" :value="h">{{ h }}</option>
          </select>
        </div>
        <div class="kv">
          <div style="color: rgba(255,255,255,.65)">source</div>
          <select v-model="mapping.source">
            <option :value="undefined">(不导入)</option>
            <option v-for="h in preview?.headers ?? []" :key="h" :value="h">{{ h }}</option>
          </select>
        </div>
      </div>

      <div class="sep" />
      <div v-if="importResult" class="pill">{{ importResult }}</div>

      <div v-if="execResult" class="row wrap" style="margin-top: 10px; gap: 10px;">
        <span class="pill">解析总行={{ execResult.total_rows }}</span>
        <span class="pill">有效={{ execResult.valid_rows }}</span>
        <span class="pill">跳过空内容={{ execResult.skipped_empty_content }}</span>
        <span class="pill">入库新增={{ execResult.inserted }}</span>
      </div>

      <div v-if="execResult" class="row wrap" style="margin-top: 12px; gap: 10px;">
        <button class="primary" :disabled="importing" @click="gotoBatchThisImport">进入批处理（仅本次导入）</button>
        <button :disabled="importing" @click="gotoBatchUnlabeled">进入批处理（未标注）</button>
      </div>

      <div class="sep" />
      <div style="font-weight: 700; margin-bottom: 8px;">预览（前 20 行）</div>
      <table class="table" v-if="preview">
        <thead>
          <tr>
            <th v-for="h in preview.headers" :key="h">{{ h }}</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="(r, idx) in preview.rows" :key="idx">
            <td v-for="h in preview.headers" :key="h">
              <span style="white-space: pre-wrap">{{ r[h] }}</span>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <div class="card" v-else>
      <div style="color: rgba(255,255,255,.65)">请选择一个文件开始导入。你也可以使用 samples/sms_samples.csv 做一键自测。</div>
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
