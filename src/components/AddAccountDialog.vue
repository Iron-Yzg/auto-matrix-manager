<script setup lang="ts">
import { onMounted, watch } from 'vue'
import { useBrowserAuth, type AuthStep } from '../composables/useBrowserAuth'

const props = defineProps<{
  show: boolean
  platform: string
}>()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'added'): void
}>()

// Use browser auth composable
const { state, isAuthenticating, startAuth, cancelAuth } = useBrowserAuth()

// Watch for dialog open/close
watch(() => props.show, (newVal) => {
  if (!newVal) {
    // Reset state when dialog closes
    if (!isAuthenticating.value) {
      state.value.step = 'idle'
      state.value.message = '准备开始授权'
      state.value.screenshot = null
      state.value.error = null
    }
  }
})

// Start auth when dialog opens
onMounted(() => {
  if (props.show && props.platform && state.value.step === 'idle') {
    // Auto-start auth when dialog opens
    startAuthForPlatform()
  }
})

const startAuthForPlatform = async () => {
  await startAuth(props.platform)
}

// Get step icon based on current step
const getStepIcon = (): string => {
  switch (state.value.step) {
    case 'idle':
      return `<svg class="w-12 h-12 text-indigo-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M21 12a9 9 0 01-9 9m9-9a9 9 0 00-9-9m9 9H3m9 9a9 9 0 01-9-9m9 9c1.657 0 3-4.03 3-9s-1.343-9-3-9m0 18c-1.657 0-3-4.03-3-9s1.343-9 3-9m-9 9a9 9 0 019-9" />
      </svg>`
    case 'launching':
    case 'opening_login':
      return `<div class="w-12 h-12 border-4 border-indigo-600 border-t-transparent rounded-full animate-spin"></div>`
    case 'waiting_login':
      return `<div class="w-12 h-12 border-4 border-amber-500 border-t-transparent rounded-full animate-spin"></div>`
    case 'login_detected':
    case 'navigating':
    case 'extracting':
    case 'closing':
      return `<div class="w-12 h-12 border-4 border-blue-500 border-t-transparent rounded-full animate-spin"></div>`
    case 'completed':
      return `<svg class="w-12 h-12 text-emerald-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
      </svg>`
    case 'error':
      return `<svg class="w-12 h-12 text-rose-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
      </svg>`
    default:
      return ''
  }
}

// Get step color class
const getStepColorClass = (): string => {
  switch (state.value.step) {
    case 'idle': return 'bg-indigo-50 border-indigo-200'
    case 'launching':
    case 'opening_login': return 'bg-indigo-50 border-indigo-200'
    case 'waiting_login': return 'bg-amber-50 border-amber-200'
    case 'login_detected':
    case 'navigating':
    case 'extracting':
    case 'closing': return 'bg-blue-50 border-blue-200'
    case 'completed': return 'bg-emerald-50 border-emerald-200'
    case 'error': return 'bg-rose-50 border-rose-200'
    default: return 'bg-slate-50 border-slate-200'
  }
}

// Get status text
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

const handleClose = () => {
  if (isAuthenticating.value) {
    cancelAuth()
  }
  emit('close')
}

// Handle completion - emit added event when completed
watch(() => state.value.step, (newStep) => {
  if (newStep === 'completed') {
    // Wait a moment then close
    setTimeout(() => {
      emit('added')
      emit('close')
    }, 1500)
  }
})
</script>

<template>
  <Teleport to="body">
    <div v-if="show" class="fixed inset-0 bg-slate-900/60 backdrop-blur-sm flex items-center justify-center z-50 p-4" @click.self="handleClose">
      <div class="bg-white rounded-2xl w-full max-w-lg shadow-2xl">
        <!-- Header -->
        <div class="px-6 py-4 border-b border-slate-100 flex items-center justify-between">
          <h2 class="text-lg font-semibold text-slate-800">添加账号</h2>
          <button @click="handleClose" class="w-8 h-8 flex items-center justify-center rounded-lg hover:bg-slate-100 transition-colors">
            <svg class="w-5 h-5 text-slate-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>

        <!-- Body -->
        <div class="p-6">
          <!-- Status Display -->
          <div :class="['rounded-2xl p-8 text-center border-2 transition-all duration-300', getStepColorClass()]">
            <!-- Step Icon -->
            <div class="flex justify-center mb-4" v-html="getStepIcon()"></div>

            <!-- Step Title -->
            <h3 class="text-lg font-semibold text-slate-800 mb-2">
              {{ getStatusText(state.step) }}
            </h3>

            <!-- Message -->
            <p class="text-sm text-slate-600 mb-4">
              {{ state.message }}
            </p>

            <!-- Screenshot (for QR code) -->
            <div v-if="state.screenshot" class="mb-4">
              <img :src="state.screenshot" alt="登录页面" class="max-w-[200px] mx-auto rounded-lg border border-slate-200" />
            </div>

            <!-- Error Details -->
            <div v-if="state.error" class="text-sm text-rose-600 mt-2">
              {{ state.error }}
            </div>
          </div>

          <!-- Help Text -->
          <div v-if="state.step === 'waiting_login'" class="bg-amber-50 border border-amber-200 rounded-xl p-4 mt-4">
            <p class="text-sm text-amber-800">
              <strong>操作提示：</strong>
            </p>
            <ol class="text-xs text-amber-700 mt-2 list-decimal list-inside space-y-1">
              <li>系统已启动无头浏览器并打开登录页面</li>
              <li>请在浏览器中完成扫码或账号密码登录</li>
              <li>登录成功后系统会自动检测并提取凭证</li>
              <li>无需手动操作，耐心等待即可</li>
            </ol>
          </div>
        </div>

        <!-- Footer -->
        <div class="px-6 py-4 border-t border-slate-100 flex gap-3 justify-end">
          <button
            v-if="state.step === 'idle'"
            @click="startAuthForPlatform"
            class="px-6 py-2.5 text-sm font-medium text-white bg-indigo-600 rounded-xl hover:bg-indigo-700 transition-colors"
          >
            开始授权
          </button>

          <button
            v-if="state.step === 'error'"
            @click="startAuthForPlatform"
            class="px-6 py-2.5 text-sm font-medium text-white bg-indigo-600 rounded-xl hover:bg-indigo-700 transition-colors"
          >
            重试
          </button>

          <button
            v-if="isAuthenticating && state.step !== 'completed'"
            @click="cancelAuth"
            class="px-6 py-2.5 text-sm font-medium text-slate-600 hover:text-slate-900 hover:bg-slate-100 rounded-xl transition-colors"
          >
            取消
          </button>

          <button
            v-if="state.step === 'completed'"
            @click="handleClose"
            class="px-6 py-2.5 text-sm font-medium text-white bg-emerald-600 rounded-xl hover:bg-emerald-700 transition-colors"
          >
            完成
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>
