<script setup lang="ts">
import { ref, onMounted, computed, watch } from 'vue'
import { PLATFORMS, type Platform } from '../types'
import { getAccounts, deleteAccount, toFrontendAccount } from '../services/api'
import { useBrowserAuth, type AuthStep } from '../composables/useBrowserAuth'

// Accounts list
const accounts = ref<Array<{
  id: string
  platform: string
  accountName: string
  avatar: string
  status: 'active' | 'expired' | 'pending'
  authorizedAt: string
}>>([])

const loading = ref(false)
const currentPlatform = ref<Platform | 'all'>('all')

// Add account modal
const showAddAccountModal = ref(false)
const selectedPlatformForAdd = ref<Platform>('douyin')

// Browser auth
const { state, isAuthenticating, startAuth, cancelAuth } = useBrowserAuth()

// Current platform info (用于下拉框显示)
const currentPlatformInfo = computed(() => {
  if (currentPlatform.value === 'all') {
    return { id: 'all', name: '全部平台', icon: '', color: '' }
  }
  return PLATFORMS.find(p => p.id === currentPlatform.value)!
})

// Get platform info helper
const getPlatformInfo = (platform: Platform) => PLATFORMS.find(p => p.id === platform)!

// Load accounts
const loadAccounts = async () => {
  loading.value = true
  try {
    let backendAccounts
    if (currentPlatform.value === 'all') {
      // 获取所有平台的账号
      backendAccounts = await getAccounts('all')
    } else {
      backendAccounts = await getAccounts(currentPlatform.value)
    }
    accounts.value = backendAccounts.map(toFrontendAccount)
  } catch (error) {
    console.error('Failed to load accounts:', error)
    accounts.value = []
  } finally {
    loading.value = false
  }
}

onMounted(() => {
  loadAccounts()
})

// Watch for auth completion to refresh account list
watch(() => state.value.step, (newStep) => {
  if (newStep === 'completed') {
    // Reload accounts after successful auth
    loadAccounts()
  }
})

// Platform changed
const handlePlatformChange = () => {
  loadAccounts()
}

// Open add account modal
const openAddAccountModal = () => {
  showAddAccountModal.value = true
  selectedPlatformForAdd.value = 'douyin'
}

// Close add account modal
const closeAddAccountModal = () => {
  showAddAccountModal.value = false
}

// Confirm add account with selected platform
const confirmAddAccount = async () => {
  closeAddAccountModal()
  await startAuth(selectedPlatformForAdd.value)
}

// Start adding account (called from modal)
const handleAddAccount = async () => {
  openAddAccountModal()
}

// Get status config
const getStatusConfig = (status: 'active' | 'expired' | 'pending') => {
  switch (status) {
    case 'active': return { text: '已授权', bg: 'bg-emerald-50', textColor: 'text-emerald-600', dot: 'bg-emerald-500' }
    case 'expired': return { text: '已过期', bg: 'bg-rose-50', textColor: 'text-rose-600', dot: 'bg-rose-500' }
    default: return { text: '待授权', bg: 'bg-amber-50', textColor: 'text-amber-600', dot: 'bg-amber-500' }
  }
}

// Handle reauthorize
const handleReauthorize = async (account: typeof accounts.value[0]) => {
  console.log('Reauthorize account:', account.id)
  currentPlatform.value = account.platform as Platform
  // 传递需要更新的账号ID，这样会更新现有账号而不是创建新账号
  await startAuth(account.platform, account.id)
}

// Handle delete
const handleDelete = async (id: string) => {
  try {
    await deleteAccount(id)
    const index = accounts.value.findIndex(a => a.id === id)
    if (index > -1) accounts.value.splice(index, 1)
  } catch (error) {
    console.error('Failed to delete account:', error)
  }
}

// Get auth step icon
const getStepIcon = (): string => {
  switch (state.value.step) {
    case 'idle':
      return `<svg class="w-6 h-6 text-indigo-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M21 12a9 9 0 01-9 9m9-9a9 9 0 00-9-9m9 9H3m9 9a9 9 0 01-9-9m9 9c1.657 0 3-4.03 3-9s-1.343-9-3-9m0 18c-1.657 0-3-4.03-3-9s1.343-9 3-9m-9 9a9 9 0 019-9" />
      </svg>`
    case 'launching':
    case 'opening_login':
      return `<div class="w-6 h-6 border-2 border-indigo-600 border-t-transparent rounded-full animate-spin"></div>`
    case 'waiting_login':
      return `<div class="w-6 h-6 border-2 border-amber-500 border-t-transparent rounded-full animate-spin"></div>`
    case 'login_detected':
    case 'navigating':
    case 'extracting':
    case 'closing':
      return `<div class="w-6 h-6 border-2 border-blue-500 border-t-transparent rounded-full animate-spin"></div>`
    case 'completed':
      return `<svg class="w-6 h-6 text-emerald-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
      </svg>`
    case 'error':
      return `<svg class="w-6 h-6 text-rose-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
      </svg>`
    default: return ''
  }
}

// Get auth status text
const getStatusText = (step: AuthStep): string => {
  const textMap: Record<AuthStep, string> = {
    'idle': '准备开始',
    'launching': '正在启动浏览器...',
    'opening_login': '正在打开登录页面...',
    'waiting_login': '等待登录',
    'login_detected': '检测到登录成功',
    'navigating': '正在跳转到上传页面...',
    'extracting': '正在提取凭证...',
    'closing': '正在关闭浏览器...',
    'completed': '授权完成',
    'error': '授权失败',
  }
  return textMap[step] || step
}
</script>

<template>
  <div class="h-full flex flex-col p-6">
    <!-- Header -->
    <div class="flex items-center justify-between mb-4 flex-shrink-0">
      <div class="flex items-center gap-4">
        <!-- Platform Selector (用于筛选列表) -->
        <div class="relative">
          <div class="flex items-center gap-2 px-3 py-2 bg-white border border-slate-200 rounded-xl hover:border-slate-300 transition-colors cursor-pointer">
            <img v-if="currentPlatform !== 'all'" :src="currentPlatformInfo.icon" :alt="currentPlatformInfo.name" class="w-5 h-5" />
            <svg v-else class="w-5 h-5 text-slate-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2V6zM14 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2V6zM4 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2v-2zM14 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2v-2z" />
            </svg>
            <span class="text-sm font-medium text-slate-700">{{ currentPlatformInfo.name }}</span>
            <svg class="w-4 h-4 text-slate-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
            </svg>
          </div>
          <select
            v-model="currentPlatform"
            @change="handlePlatformChange"
            class="absolute inset-0 w-full h-full opacity-0 cursor-pointer"
          >
            <option value="all">全部平台</option>
            <option v-for="p in PLATFORMS" :key="p.id" :value="p.id">{{ p.name }}</option>
          </select>
        </div>

        <!-- Auth Status (when authenticating) -->
        <div v-if="isAuthenticating" class="flex items-center gap-3 px-4 py-3 bg-gradient-to-r from-amber-50 to-orange-50 border border-amber-200 rounded-xl shadow-sm">
          <div v-html="getStepIcon()"></div>
          <div class="flex flex-col">
            <span class="text-sm font-semibold text-amber-800">{{ getStatusText(state.step) }}</span>
            <span class="text-xs text-amber-600">{{ state.message }}</span>
          </div>
          <button @click="cancelAuth" class="ml-2 px-3 py-1.5 text-xs font-medium text-amber-600 hover:text-amber-800 hover:bg-amber-100 rounded-lg transition-colors">
            取消
          </button>
        </div>
      </div>

      <!-- Add Account Button -->
      <button
        v-if="!isAuthenticating"
        @click="handleAddAccount"
        class="group flex items-center gap-2 px-4 py-2.5 bg-indigo-600 text-white rounded-xl hover:bg-indigo-700 transition-all duration-200 shadow-sm hover:shadow-md"
      >
        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
        </svg>
        <span class="font-medium">添加账号</span>
      </button>
    </div>

    <!-- Auth Error -->
    <div v-if="state.error" class="mb-4 p-4 bg-rose-50 border border-rose-200 rounded-xl">
      <p class="text-sm text-rose-700">{{ state.error }}</p>
    </div>

    <!-- Account Table -->
    <div class="flex-1 bg-white rounded-2xl border border-slate-200 overflow-hidden flex flex-col">
      <div v-if="loading" class="flex-1 flex items-center justify-center">
        <div class="w-8 h-8 border-4 border-indigo-500 border-t-transparent rounded-full animate-spin"></div>
      </div>
      <div v-else class="overflow-y-auto flex-1">
        <table class="w-full">
          <thead class="bg-slate-50 border-b border-slate-200 sticky top-0">
            <tr>
              <th class="text-left px-6 py-3 text-xs font-semibold text-slate-500 uppercase tracking-wider">平台</th>
              <th class="text-left px-6 py-3 text-xs font-semibold text-slate-500 uppercase tracking-wider">账号名称</th>
              <th class="text-left px-6 py-3 text-xs font-semibold text-slate-500 uppercase tracking-wider">授权时间</th>
              <th class="text-left px-6 py-3 text-xs font-semibold text-slate-500 uppercase tracking-wider">状态</th>
              <th class="text-right px-6 py-3 text-xs font-semibold text-slate-500 uppercase tracking-wider">操作</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-slate-100">
            <tr v-for="account in accounts" :key="account.id" class="hover:bg-slate-50/80 transition-colors">
              <td class="px-6 py-3">
                <div class="flex items-center gap-3">
                  <div class="w-8 h-8 rounded-lg bg-slate-100 flex items-center justify-center overflow-hidden">
                    <img :src="getPlatformInfo(account.platform as Platform).icon" :alt="getPlatformInfo(account.platform as Platform).name" class="w-5 h-5 object-contain" />
                  </div>
                  <span class="text-sm font-medium text-slate-700">{{ getPlatformInfo(account.platform as Platform).name }}</span>
                </div>
              </td>
              <td class="px-6 py-3">
                <div class="flex items-center gap-3">
                  <img :src="account.avatar" :alt="account.accountName" class="w-7 h-7 rounded-full bg-slate-100" />
                  <span class="text-sm text-slate-700">{{ account.accountName }}</span>
                </div>
              </td>
              <td class="px-6 py-3 text-sm text-slate-500">{{ account.authorizedAt }}</td>
              <td class="px-6 py-3">
                <span :class="['inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium', getStatusConfig(account.status).bg, getStatusConfig(account.status).textColor]">
                  <span :class="['w-1.5 h-1.5 rounded-full', getStatusConfig(account.status).dot]"></span>
                  {{ getStatusConfig(account.status).text }}
                </span>
              </td>
              <td class="px-6 py-3">
                <div class="flex items-center justify-end gap-2">
                  <button @click="handleReauthorize(account)" class="px-3 py-1.5 text-xs font-medium text-slate-600 hover:text-slate-900 hover:bg-slate-100 rounded-lg transition-colors">重新授权</button>
                  <button @click="handleDelete(account.id)" class="px-3 py-1.5 text-xs font-medium text-rose-600 hover:text-rose-700 hover:bg-rose-50 rounded-lg transition-colors">删除</button>
                </div>
              </td>
            </tr>
          </tbody>
        </table>
      </div>

      <!-- Empty State -->
      <div v-if="!loading && accounts.length === 0" class="py-12 text-center">
        <div class="w-16 h-16 mx-auto mb-3 rounded-full bg-slate-100 flex items-center justify-center">
          <svg class="w-8 h-8 text-slate-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0zm6 3a2 2 0 11-4 0 2 2 0 014 0zM7 10a2 2 0 11-4 0 2 2 0 014 0z" />
          </svg>
        </div>
        <h3 class="text-sm font-medium text-slate-600 mb-1">暂无账号</h3>
        <p class="text-xs text-slate-400">点击"添加账号"按钮开始授权</p>
      </div>
    </div>
  </div>

  <!-- Add Account Platform Selection Modal -->
  <div v-if="showAddAccountModal" class="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
    <div class="bg-white rounded-xl p-6 max-w-sm mx-4 shadow-xl">
      <h3 class="text-lg font-semibold text-slate-800 text-center mb-4">选择平台</h3>
      <div class="space-y-2">
        <button
          v-for="platform in PLATFORMS"
          :key="platform.id"
          @click="selectedPlatformForAdd = platform.id as Platform"
          :class="[
            'w-full flex items-center gap-3 px-4 py-3 rounded-xl transition-all duration-200',
            selectedPlatformForAdd === platform.id
              ? 'bg-indigo-50 border-2 border-indigo-600'
              : 'bg-white border-2 border-slate-200 hover:border-slate-300'
          ]"
        >
          <img :src="platform.icon" :alt="platform.name" class="w-6 h-6 object-contain" />
          <span :class="['font-medium', selectedPlatformForAdd === platform.id ? 'text-indigo-600' : 'text-slate-700']">
            {{ platform.name }}
          </span>
          <svg v-if="selectedPlatformForAdd === platform.id" class="w-5 h-5 text-indigo-600 ml-auto" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
          </svg>
        </button>
      </div>
      <div class="flex gap-3 mt-6">
        <button @click="closeAddAccountModal" class="flex-1 px-4 py-2.5 text-slate-600 font-medium bg-slate-100 rounded-xl hover:bg-slate-200 transition-colors">
          取消
        </button>
        <button @click="confirmAddAccount" class="flex-1 px-4 py-2.5 text-white font-medium bg-indigo-600 rounded-xl hover:bg-indigo-700 transition-colors">
          确定
        </button>
      </div>
    </div>
  </div>
</template>
