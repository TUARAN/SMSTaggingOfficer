export type Industry = '金融' | '通用' | '政务' | '渠道' | '互联网' | '其他'

export type SmsType =
  | '验证码'
  | '交易提醒'
  | '账单催缴'
  | '保险续保'
  | '物流取件'
  | '会员账号变更'
  | '政务通知'
  | '风险提示'
  | '营销推广'
  | '其他'

export type Entities = {
  brand: string | null
  verification_code: string | null
  amount: number | null
  balance: number | null
  account_suffix: string | null
  time_text: string | null
  url: string | null
  phone_in_text: string | null
}

export type LabelOutput = {
  industry: Industry
  type: SmsType
  entities: Entities
  confidence: number
  needs_review: boolean
  reasons: string[]
  signals?: Record<string, any>
  rules_version: string
  model_version: string
  schema_version: string
}

export type MessageRow = {
  id: number
  content: string
  received_at: string | null
  sender: string | null
  phone: string | null
  source: string | null
  has_url: boolean
  has_amount: boolean
  has_verification_code: boolean
  label: LabelOutput | null
}
