<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'

interface ExtractorRule {
  field: string  // 固定字段名: nickname, avatar_url, sec_uid, third_id
  rule: string   // 提取规则
}

interface ExtractorConfig {
  id: string
  platform_id: string
  platform_name: string
  login_url: string
  login_success_pattern: string
  redirect_url: string | null
  extract_rules: {
    user_info: Record<string, string>
    request_headers: Record<string, string>
    local_storage: string[]
    cookie?: {
      source: string
      api_path?: string
      header_name?: string
    }
  }
  is_default: boolean
}

const props = defineProps<{
  platformId: string
  platformName: string
}>()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'saved'): void
}>()

const loading = ref(false)
const saving = ref(false)

const config = ref<ExtractorConfig>({
  id: '',
  platform_id: props.platformId,
  platform_name: props.platformName,
  login_url: '',
  login_success_pattern: '',
  redirect_url: null,
  extract_rules: {
    user_info: {},
    request_headers: {},
    local_storage: []
  },
  is_default: false
})

// 用户信息提取规则 - 固定字段
const userInfoFields = [
  { key: 'nickname', label: '昵称', desc: '用户昵称' },
  { key: 'avatar_url', label: '头像', desc: '用户头像 URL' },
  { key: 'third_id', label: '用户ID', desc: '用户唯一标识' },
  { key: 'sec_uid', label: 'SecUid', desc: '安全用户ID（部分平台有）' },
]

// 用户信息提取规则 - 使用 Map 存储以保持顺序
const userInfoRules = ref<Map<string, string>>(new Map())

// 请求头提取规则
const headerRules = ref<ExtractorRule[]>([])

// LocalStorage 键
const localStorageKeys = ref<string[]>([])

// Cookie 设置
const cookieSource = ref<'browser' | 'api'>('browser')
const cookieApiPath = ref('')
const cookieHeaderName = ref('')

// 加载配置
onMounted(async () => {
  loading.value = true
  try {
    const result = await invoke<Record<string, unknown> | null>('get_extractor_config', {
      platformId: props.platformId
    })

    if (result) {
      config.value = result as unknown as ExtractorConfig

      // 解析规则
      const rules = result.extract_rules as ExtractorConfig['extract_rules']

      // 解析用户信息规则 - 填充到 Map
      const userInfoMap = new Map<string, string>()
      for (const field of userInfoFields) {
        const rule = rules.user_info?.[field.key]
        if (rule) {
          userInfoMap.set(field.key, rule)
        } else {
          userInfoMap.set(field.key, '')
        }
      }
      userInfoRules.value = userInfoMap

      headerRules.value = Object.entries(rules.request_headers || {}).map(([field, rule]) => ({
        field,
        rule
      }))

      localStorageKeys.value = [...(rules.local_storage || [])]

      if (rules.cookie) {
        cookieSource.value = rules.cookie.source === 'from_api' ? 'api' : 'browser'
        cookieApiPath.value = rules.cookie.api_path || ''
        cookieHeaderName.value = rules.cookie.header_name || ''
      }
    } else {
      // 没有配置时初始化空规则
      for (const field of userInfoFields) {
        userInfoRules.value.set(field.key, '')
      }
    }
  } catch (error) {
    console.error('Failed to load config:', error)
  } finally {
    loading.value = false
  }
})

// 添加请求头规则
const addHeaderRule = () => {
  headerRules.value.push({ field: '', rule: '' })
}

// 删除请求头规则
const removeHeaderRule = (index: number) => {
  headerRules.value.splice(index, 1)
}

// 添加 localStorage 键
const addLocalStorageKey = () => {
  localStorageKeys.value.push('')
}

// 删除 localStorage 键
const removeLocalStorageKey = (index: number) => {
  localStorageKeys.value.splice(index, 1)
}

// 保存配置
const handleSave = async () => {
  saving.value = true
  try {
    // 构建 extract_rules
    const extractRules: Record<string, unknown> = {
      user_info: {},
      request_headers: {},
      local_storage: localStorageKeys.value.filter(k => k.trim()),
      cookie: {
        source: cookieSource.value === 'api' ? 'from_api' : 'from_browser'
      }
    }

    // 保存用户信息规则 - 从 Map 读取
    for (const field of userInfoFields) {
      const rule = userInfoRules.value.get(field.key)
      if (rule && rule.trim()) {
        (extractRules.user_info as Record<string, string>)[field.key] = rule.trim()
      }
    }

    for (const r of headerRules.value) {
      if (r.field.trim() && r.rule.trim()) {
        (extractRules.request_headers as Record<string, string>)[r.field.trim()] = r.rule.trim()
      }
    }

    if (cookieSource.value === 'api') {
      (extractRules.cookie as Record<string, string>).api_path = cookieApiPath.value.trim()
      ;(extractRules.cookie as Record<string, string>).header_name = cookieHeaderName.value.trim() || 'cookie'
    }

    await invoke('save_extractor_config', {
      platformId: config.value.platform_id,
      platformName: config.value.platform_name,
      loginUrl: config.value.login_url,
      loginSuccessPattern: config.value.login_success_pattern,
      redirectUrl: config.value.redirect_url || null,
      extractRules: JSON.stringify(extractRules)
    })

    emit('saved')
    emit('close')
  } catch (error) {
    console.error('Failed to save config:', error)
    alert('保存失败: ' + error)
  } finally {
    saving.value = false
  }
}

// 规则语法提示
const ruleSyntaxTips = [
  { pattern: '${api:/path/to/api:response:body:user:uid}', desc: '从 API 响应 body 提取字段' },
  { pattern: '${api:/path/to/api:request:headers:cookie}', desc: '从 API 请求头提取字段' },
  { pattern: '${localStorage:key}', desc: '从 localStorage 提取值' },
  { pattern: '固定值', desc: '直接返回固定字符串' }
]
</script>

<template>
  <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50" @click.self="emit('close')">
    <div class="bg-white rounded-2xl w-full max-w-4xl max-h-[90vh] overflow-hidden flex flex-col">
      <!-- 头部 -->
      <div class="px-6 py-4 border-b border-slate-200 flex items-center justify-between">
        <div>
          <h2 class="text-lg font-semibold text-slate-800">数据提取引擎配置</h2>
          <p class="text-sm text-slate-500 mt-0.5">{{ platformName }}</p>
        </div>
        <button @click="emit('close')" class="text-slate-400 hover:text-slate-600">
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </div>

      <!-- 内容 -->
      <div class="flex-1 overflow-y-auto p-6">
        <div v-if="loading" class="flex items-center justify-center py-12">
          <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-indigo-600"></div>
        </div>

        <template v-else>
          <!-- 基本设置 -->
          <div class="mb-6">
            <h3 class="text-sm font-semibold text-slate-800 mb-3">基本设置</h3>
            <div class="grid gap-4">
              <div>
                <label class="block text-xs font-medium text-slate-600 mb-1.5">登录页 URL</label>
                <input
                  v-model="config.login_url"
                  type="text"
                  placeholder="https://creator.douyin.com/"
                  class="w-full px-3 py-2 bg-slate-50 border border-slate-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-indigo-500/10 focus:bg-white"
                />
              </div>
              <div>
                <label class="block text-xs font-medium text-slate-600 mb-1.5">登录成功 URL 模式</label>
                <input
                  v-model="config.login_success_pattern"
                  type="text"
                  placeholder="**/creator-micro/**"
                  class="w-full px-3 py-2 bg-slate-50 border border-slate-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-indigo-500/10 focus:bg-white"
                />
                <p class="text-xs text-slate-400 mt-1">支持 glob 格式，如 **/creator-micro/**</p>
              </div>
              <div>
                <label class="block text-xs font-medium text-slate-600 mb-1.5">跳转页 URL（可选）</label>
                <input
                  v-model="config.redirect_url"
                  type="text"
                  placeholder="登录成功后跳转的页面"
                  class="w-full px-3 py-2 bg-slate-50 border border-slate-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-indigo-500/10 focus:bg-white"
                />
              </div>
            </div>
          </div>

          <!-- 规则语法提示 -->
          <div class="mb-6 p-4 bg-indigo-50 rounded-xl">
            <h3 class="text-xs font-semibold text-indigo-800 mb-2">规则语法</h3>
            <div class="grid gap-2">
              <div v-for="tip in ruleSyntaxTips" :key="tip.pattern" class="flex items-center gap-2 text-xs">
                <code class="px-1.5 py-0.5 bg-white rounded text-indigo-600 font-mono">{{ tip.pattern }}</code>
                <span class="text-indigo-600/70">{{ tip.desc }}</span>
              </div>
            </div>
          </div>

          <!-- 用户信息提取规则 - 固定字段 -->
          <div class="mb-6">
            <div class="flex items-center justify-between mb-3">
              <div>
                <h3 class="text-sm font-semibold text-slate-800">用户信息提取规则</h3>
                <p class="text-xs text-slate-400 mt-0.5">以下字段为系统保留字段，请配置对应的提取规则</p>
              </div>
            </div>
            <div class="space-y-3">
              <div v-for="field in userInfoFields" :key="field.key" class="flex items-center gap-3">
                <!-- 固定字段名（只读） -->
                <div class="w-24 flex-shrink-0">
                  <div class="text-sm font-medium text-slate-700">{{ field.label }}</div>
                  <div class="text-xs text-slate-400">{{ field.desc }}</div>
                </div>
                <!-- 规则输入框 -->
                <input
                  :value="userInfoRules.get(field.key)"
                  @input="userInfoRules.set(field.key, ($event.target as HTMLInputElement).value)"
                  type="text"
                  :placeholder="`\${api:...} 或固定值`"
                  class="flex-1 px-3 py-2 bg-slate-50 border border-slate-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-indigo-500/10 focus:bg-white"
                />
              </div>
            </div>
          </div>

          <!-- 请求头提取规则 -->
          <div class="mb-6">
            <div class="flex items-center justify-between mb-3">
              <h3 class="text-sm font-semibold text-slate-800">请求头提取规则</h3>
              <button
                @click="addHeaderRule"
                class="text-xs px-2 py-1 bg-indigo-50 text-indigo-600 rounded hover:bg-indigo-100"
              >
                + 添加规则
              </button>
            </div>
            <div class="space-y-2">
              <div v-for="(rule, index) in headerRules" :key="index" class="flex items-center gap-2">
                <input
                  v-model="rule.field"
                  type="text"
                  placeholder="头名称"
                  class="flex-1 px-3 py-2 bg-slate-50 border border-slate-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-indigo-500/10"
                />
                <input
                  v-model="rule.rule"
                  type="text"
                  placeholder="${api:...}"
                  class="flex-1 px-3 py-2 bg-slate-50 border border-slate-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-indigo-500/10"
                />
                <button
                  @click="removeHeaderRule(index)"
                  class="p-2 text-slate-400 hover:text-red-500"
                >
                  <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                  </svg>
                </button>
              </div>
              <div v-if="headerRules.length === 0" class="text-center py-4 text-slate-400 text-sm">
                暂无规则，点击上方按钮添加
              </div>
            </div>
          </div>

          <!-- LocalStorage 键 -->
          <div class="mb-6">
            <div class="flex items-center justify-between mb-3">
              <h3 class="text-sm font-semibold text-slate-800">LocalStorage 键</h3>
              <button
                @click="addLocalStorageKey"
                class="text-xs px-2 py-1 bg-indigo-50 text-indigo-600 rounded hover:bg-indigo-100"
              >
                + 添加键
              </button>
            </div>
            <div class="flex flex-wrap gap-2">
              <div v-for="(key, index) in localStorageKeys" :key="key" class="flex items-center gap-1">
                <input
                  v-model="localStorageKeys[index]"
                  type="text"
                  placeholder="localStorage key"
                  class="px-3 py-1.5 bg-slate-50 border border-slate-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-indigo-500/10"
                />
                <button
                  @click="removeLocalStorageKey(index)"
                  class="p-1 text-slate-400 hover:text-red-500"
                >
                  <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                  </svg>
                </button>
              </div>
              <div v-if="localStorageKeys.length === 0" class="text-sm text-slate-400 py-1">
                暂无键，点击上方按钮添加
              </div>
            </div>
          </div>

          <!-- Cookie 设置 -->
          <div class="mb-6">
            <h3 class="text-sm font-semibold text-slate-800 mb-3">Cookie 获取方式</h3>
            <div class="space-y-3">
              <label class="flex items-center gap-2 cursor-pointer">
                <input type="radio" v-model="cookieSource" value="browser" class="text-indigo-600" />
                <span class="text-sm text-slate-600">从浏览器直接获取</span>
              </label>
              <label class="flex items-center gap-2 cursor-pointer">
                <input type="radio" v-model="cookieSource" value="api" class="text-indigo-600" />
                <span class="text-sm text-slate-600">从 API 请求头获取</span>
              </label>
              <div v-if="cookieSource === 'api'" class="ml-6 grid gap-3 mt-2">
                <div>
                  <label class="block text-xs font-medium text-slate-600 mb-1">API 路径</label>
                  <input
                    v-model="cookieApiPath"
                    type="text"
                    placeholder="/account/api/v1/user/account/info"
                    class="w-full px-3 py-2 bg-slate-50 border border-slate-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-indigo-500/10"
                  />
                </div>
                <div>
                  <label class="block text-xs font-medium text-slate-600 mb-1">头名称</label>
                  <input
                    v-model="cookieHeaderName"
                    type="text"
                    placeholder="cookie"
                    class="w-full px-3 py-2 bg-slate-50 border border-slate-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-indigo-500/10"
                  />
                </div>
              </div>
            </div>
          </div>
        </template>
      </div>

      <!-- 底部 -->
      <div class="px-6 py-4 border-t border-slate-200 flex justify-end gap-3">
        <button
          @click="emit('close')"
          class="px-4 py-2 bg-slate-100 text-slate-600 text-sm font-medium rounded-lg hover:bg-slate-200"
        >
          取消
        </button>
        <button
          @click="handleSave"
          :disabled="saving"
          class="px-4 py-2 bg-indigo-600 text-white text-sm font-medium rounded-lg hover:bg-indigo-700 disabled:opacity-50"
        >
          {{ saving ? '保存中...' : '保存配置' }}
        </button>
      </div>
    </div>
  </div>
</template>
