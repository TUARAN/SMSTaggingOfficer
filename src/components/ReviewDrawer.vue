<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/tauri'
import type { Entities, Industry, LabelOutput, MessageRow, SmsType } from '../types'

const props = defineProps<{ open: boolean; row: MessageRow }>()
const emit = defineEmits<{ (e: 'close'): void; (e: 'saved'): void }>()

const industries: Industry[] = ['金融', '通用', '政务', '渠道', '互联网', '其他']
const types: SmsType[] = ['验证码','交易提醒','账单催缴','保险续保','物流取件','会员账号变更','政务通知','风险提示','营销推广','其他']

const saving = ref(false)
const operator = ref('reviewer')

const form = ref<LabelOutput | null>(null)

watch(
  () => props.row,
  () => {
    const base: LabelOutput = props.row.label ?? {
      industry: '其他',
      type: '其他',
      confidence: 0.5,
      needs_review: true,
      reasons: ['manual_init'],
      rules_version: 'rules_v1',
      model_version: 'n/a',
      schema_version: 'schema_v1',
      entities: {
        brand: null,
        verification_code: null,
        amount: null,
        balance: null,
        account_suffix: null,
        time_text: null,
        url: null,
        phone_in_text: null
      }
    }
    form.value = JSON.parse(JSON.stringify(base))
  },
  { immediate: true }
)

const reasonsText = computed({
  get() {
    return (form.value?.reasons ?? []).join('\n')
  },
  set(v: string) {
    if (!form.value) return
    form.value.reasons = v
      .split(/\r?\n/)
      .map((s) => s.trim())
      .filter(Boolean)
  }
})

function close() {
  emit('close')
}

async function save() {
  if (!form.value) return
  saving.value = true
  try {
    await invoke('label_update_manual', {
      message_id: props.row.id,
      operator: operator.value,
      new_label: form.value
    })
    emit('saved')
  } finally {
    saving.value = false
  }
}

function setNull(obj: Entities, key: keyof Entities) {
  obj[key] = null
}
</script>

<template>
  <div class="mask" v-if="open" @click.self="close">
    <div class="drawer">
      <div class="row" style="justify-content: space-between; align-items: center;">
        <div>
          <div style="font-weight: 800; font-size: 16px;">复核 · ID {{ row.id }}</div>
          <div style="color: rgba(255,255,255,.65); font-size: 12px; margin-top: 4px;">任何修改都会写入 audit_logs（前后差异 + 操作者）。</div>
        </div>
        <div class="row">
          <button :disabled="saving" @click="close">关闭</button>
          <button class="primary" :disabled="saving" @click="save">保存</button>
        </div>
      </div>

      <div class="sep" />

      <div class="card">
        <div style="font-weight: 700;">短信内容</div>
        <div style="white-space: pre-wrap; margin-top: 8px; color: rgba(255,255,255,.9)">{{ row.content }}</div>
      </div>

      <div class="sep" />

      <div class="grid" v-if="form">
        <div class="card">
          <div style="font-weight: 700; margin-bottom: 10px;">标签</div>
          <div class="kv">
            <div style="color: rgba(255,255,255,.65)">industry</div>
            <select v-model="form.industry">
              <option v-for="i in industries" :key="i" :value="i">{{ i }}</option>
            </select>
          </div>
          <div class="kv" style="margin-top: 10px;">
            <div style="color: rgba(255,255,255,.65)">type</div>
            <select v-model="form.type">
              <option v-for="t in types" :key="t" :value="t">{{ t }}</option>
            </select>
          </div>
          <div class="kv" style="margin-top: 10px;">
            <div style="color: rgba(255,255,255,.65)">confidence</div>
            <input type="number" step="0.01" min="0" max="1" v-model.number="form.confidence" />
          </div>
          <div class="kv" style="margin-top: 10px;">
            <div style="color: rgba(255,255,255,.65)">needs_review</div>
            <select v-model="form.needs_review">
              <option :value="true">true</option>
              <option :value="false">false</option>
            </select>
          </div>
          <div class="kv" style="margin-top: 10px;">
            <div style="color: rgba(255,255,255,.65)">operator</div>
            <input v-model="operator" placeholder="reviewer" />
          </div>
        </div>

        <div class="card">
          <div style="font-weight: 700; margin-bottom: 10px;">实体（缺失填 null）</div>
          <div class="entityGrid">
            <div class="kv" v-for="k in Object.keys(form.entities)" :key="k">
              <div class="mono" style="color: rgba(255,255,255,.65)">{{ k }}</div>
              <div class="row" style="gap: 8px;">
                <input
                  v-model="(form.entities as any)[k]"
                  :placeholder="'null'"
                />
                <button @click="setNull(form.entities, k as any)">置空</button>
              </div>
            </div>
          </div>
        </div>

        <div class="card" style="grid-column: 1 / -1;">
          <div style="font-weight: 700; margin-bottom: 10px;">reasons（每行一条）</div>
          <textarea rows="5" v-model="reasonsText" class="mono" />
          <div style="color: rgba(255,255,255,.65); font-size: 12px; margin-top: 8px;">
            rules_version={{ form.rules_version }} · model_version={{ form.model_version }} · schema_version={{ form.schema_version }}
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.mask {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.55);
  backdrop-filter: blur(6px);
  display: flex;
  justify-content: flex-end;
}
.drawer {
  width: min(860px, 92vw);
  height: 100vh;
  overflow: auto;
  border-left: 1px solid rgba(255, 255, 255, 0.10);
  background: rgba(11, 16, 32, 0.95);
  padding: 16px;
}
.grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 12px;
}
@media (max-width: 980px) {
  .grid { grid-template-columns: 1fr; }
}
.entityGrid {
  display: grid;
  grid-template-columns: 1fr;
  gap: 10px;
}
</style>
