<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/tauri'
import { useRoute } from 'vue-router'
import ReviewDrawer from '../components/ReviewDrawer.vue'

import type { Entities, Industry, LabelOutput, MessageRow, SmsType } from '../types'

type ListQuery = {
  industry?: Industry | null
  sms_type?: SmsType | null
  needs_review?: boolean | null
  conf_min?: number | null
  conf_max?: number | null
  has_url?: boolean | null
  has_verification_code?: boolean | null
  has_amount?: boolean | null
  q?: string | null
  limit: number
  offset: number
}

const industries: Industry[] = ['金融', '通用', '政务', '渠道', '互联网', '其他']
const types: SmsType[] = ['验证码','交易提醒','账单催缴','保险续保','物流取件','会员账号变更','政务通知','风险提示','营销推广','其他']

const query = ref<ListQuery>({ limit: 50, offset: 0 })
const rows = ref<MessageRow[]>([])
const total = ref(0)
const loading = ref(false)

const route = useRoute()

const selected = ref<MessageRow | null>(null)
const drawerOpen = ref(false)

const page = computed(() => Math.floor(query.value.offset / query.value.limit) + 1)

async function load() {
  loading.value = true
  try {
    const res = await invoke<{ total: number; rows: MessageRow[] }>('messages_list', { query: query.value })
    total.value = res.total
    rows.value = res.rows
  } finally {
    loading.value = false
  }
}

function openDrawer(row: MessageRow) {
  selected.value = row
  drawerOpen.value = true
}

async function onSaved() {
  drawerOpen.value = false
  selected.value = null
  await load()
}

function prev() {
  query.value.offset = Math.max(0, query.value.offset - query.value.limit)
}

function next() {
  if (query.value.offset + query.value.limit >= total.value) return
  query.value.offset += query.value.limit
}

function applyRoutePrefill() {
  const v = route.query.needs_review
  if (v === undefined) return
  const s = Array.isArray(v) ? v[0] : v
  if (s === '1' || s === 'true') query.value.needs_review = true
  else if (s === '0' || s === 'false') query.value.needs_review = false
  else query.value.needs_review = null
}

watch(
  () => ({ ...query.value, offset: undefined, limit: undefined }),
  async () => {
    const wasZero = query.value.offset === 0
    query.value.offset = 0
    if (wasZero) await load()
  },
  { deep: true }
)

watch(
  () => [query.value.offset, query.value.limit],
  async () => {
    await load()
  }
)

watch(
  () => route.query.needs_review,
  async () => {
    applyRoutePrefill()
    const wasZero = query.value.offset === 0
    query.value.offset = 0
    if (wasZero) await load()
  }
)

onMounted(async () => {
  applyRoutePrefill()
  await load()
})
</script>

<template>
  <div class="page">
    <div class="row wrap" style="justify-content: space-between; align-items: flex-end;">
      <div>
        <h2 style="margin: 0;">列表 / 复核</h2>
        <div style="color: rgba(255,255,255,.65); margin-top: 6px;">支持筛选行业、类型、needs_review、置信度区间、含链接/验证码/金额，点击行进入复核抽屉。</div>
      </div>
      <div class="pill">总数：{{ total }}</div>
    </div>

    <div class="sep" />

    <div class="card">
      <div class="filters">
        <div class="kv">
          <div style="color: rgba(255,255,255,.65)">行业</div>
          <select v-model="query.industry">
            <option :value="null">全部</option>
            <option v-for="i in industries" :key="i" :value="i">{{ i }}</option>
          </select>
        </div>
        <div class="kv">
          <div style="color: rgba(255,255,255,.65)">类型</div>
          <select v-model="query.sms_type">
            <option :value="null">全部</option>
            <option v-for="t in types" :key="t" :value="t">{{ t }}</option>
          </select>
        </div>
        <div class="kv">
          <div style="color: rgba(255,255,255,.65)">needs_review</div>
          <select v-model="query.needs_review">
            <option :value="null">全部</option>
            <option :value="true">是</option>
            <option :value="false">否</option>
          </select>
        </div>
        <div class="kv">
          <div style="color: rgba(255,255,255,.65)">置信度≥</div>
          <input type="number" min="0" max="1" step="0.01" v-model.number="query.conf_min" />
        </div>
        <div class="kv">
          <div style="color: rgba(255,255,255,.65)">置信度≤</div>
          <input type="number" min="0" max="1" step="0.01" v-model.number="query.conf_max" />
        </div>
        <div class="kv">
          <div style="color: rgba(255,255,255,.65)">含链接</div>
          <select v-model="query.has_url">
            <option :value="null">不限</option>
            <option :value="true">是</option>
            <option :value="false">否</option>
          </select>
        </div>
        <div class="kv">
          <div style="color: rgba(255,255,255,.65)">含验证码</div>
          <select v-model="query.has_verification_code">
            <option :value="null">不限</option>
            <option :value="true">是</option>
            <option :value="false">否</option>
          </select>
        </div>
        <div class="kv">
          <div style="color: rgba(255,255,255,.65)">含金额</div>
          <select v-model="query.has_amount">
            <option :value="null">不限</option>
            <option :value="true">是</option>
            <option :value="false">否</option>
          </select>
        </div>
        <div class="kv" style="grid-column: 1 / -1;">
          <div style="color: rgba(255,255,255,.65)">关键词</div>
          <input placeholder="搜索 content/sender/source" v-model="query.q" />
        </div>
      </div>

      <div class="sep" />

      <table class="table">
        <thead>
          <tr>
            <th style="width: 56px">ID</th>
            <th>content</th>
            <th style="width: 120px">industry</th>
            <th style="width: 140px">type</th>
            <th style="width: 110px">confidence</th>
            <th style="width: 120px">brand</th>
            <th style="width: 150px">key entities</th>
            <th style="width: 130px">flags</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="r in rows" :key="r.id" class="rowHover" @click="openDrawer(r)">
            <td class="mono" style="color: rgba(255,255,255,.65)">{{ r.id }}</td>
            <td>
              <div style="white-space: pre-wrap">{{ r.content }}</div>
              <div style="color: rgba(255,255,255,.55); font-size: 12px; margin-top: 6px;">
                <span v-if="r.sender">sender: {{ r.sender }}</span>
                <span v-if="r.source" style="margin-left: 10px;">source: {{ r.source }}</span>
                <span v-if="r.received_at" style="margin-left: 10px;">at: {{ r.received_at }}</span>
              </div>
            </td>
            <td>
              <span v-if="r.label" class="badge">{{ r.label.industry }}</span>
              <span v-else class="badge bad">未标注</span>
            </td>
            <td>
              <span v-if="r.label" class="badge">{{ r.label.type }}</span>
              <span v-else class="badge bad">-</span>
            </td>
            <td>
              <span v-if="r.label" class="badge" :class="r.label.confidence >= 0.85 ? 'good' : (r.label.confidence >= 0.65 ? 'warn' : 'bad')">
                {{ r.label.confidence.toFixed(2) }}
              </span>
              <span v-else class="badge bad">-</span>
            </td>
            <td>
              <span class="mono" style="color: rgba(255,255,255,.75)">{{ r.label?.entities.brand ?? '-' }}</span>
            </td>
            <td>
              <div class="mono" style="font-size: 12px; color: rgba(255,255,255,.7)">
                code={{ r.label?.entities.verification_code ?? '-' }}
                <span style="margin-left: 8px;">amt={{ r.label?.entities.amount ?? '-' }}</span>
              </div>
              <div class="mono" style="font-size: 12px; color: rgba(255,255,255,.7); margin-top: 4px;">
                url={{ r.label?.entities.url ? 'Y' : 'N' }}
                <span style="margin-left: 8px;">phone={{ r.label?.entities.phone_in_text ?? '-' }}</span>
              </div>
            </td>
            <td>
              <span class="badge" :class="r.label?.needs_review ? 'warn' : ''">needs_review={{ r.label?.needs_review ? 'Y' : 'N' }}</span>
            </td>
          </tr>
        </tbody>
      </table>

      <div class="sep" />

      <div class="row" style="justify-content: space-between;">
        <div class="pill">第 {{ page }} 页 · 每页 {{ query.limit }} 条</div>
        <div class="row">
          <button :disabled="query.offset === 0 || loading" @click="prev">上一页</button>
          <button :disabled="query.offset + query.limit >= total || loading" @click="next">下一页</button>
        </div>
      </div>
    </div>

    <ReviewDrawer
      v-if="selected"
      :open="drawerOpen"
      :row="selected"
      @close="drawerOpen = false"
      @saved="onSaved"
    />
  </div>
</template>

<style scoped>
.page { display: flex; flex-direction: column; gap: 12px; }
.filters {
  display: grid;
  grid-template-columns: 1fr 1fr 1fr;
  gap: 10px 14px;
}
@media (max-width: 1080px) {
  .filters { grid-template-columns: 1fr; }
}
.rowHover { cursor: pointer; }
.rowHover:hover { background: rgba(255, 255, 255, 0.03); }
</style>
