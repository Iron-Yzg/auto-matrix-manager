// Browser Automation Composable
// 浏览器自动化Composable - headless_chrome 自动轮询

import { ref, onUnmounted } from 'vue'
import { startBrowserAuth, checkBrowserAuthStatus, cancelBrowserAuth } from '../services/api'

// Auth state
export type AuthStep = 'idle' | 'launching' | 'opening_login' | 'waiting_login' | 'login_detected' | 'navigating' | 'extracting' | 'closing' | 'completed' | 'error'

export interface BrowserAuthState {
  step: AuthStep
  message: string
  currentUrl: string
  screenshot: string | null
  nickname: string
  avatarUrl: string
  thirdId: string
  secUid: string
  error: string | null
}

export function useBrowserAuth() {
  const state = ref<BrowserAuthState>({
    step: 'idle',
    message: '准备开始授权',
    currentUrl: '',
    screenshot: null,
    nickname: '',
    avatarUrl: '',
    thirdId: '',
    secUid: '',
    error: null,
  })

  const isAuthenticating = ref(false)
  const currentPlatform = ref('')

  // Polling timer
  let pollTimer: number | null = null

  const getAuthStep = (backendStep: string): AuthStep => {
    const stepMap: Record<string, AuthStep> = {
      'Idle': 'idle',
      'LaunchingBrowser': 'launching',
      'OpeningLoginPage': 'opening_login',
      'WaitingForLogin': 'waiting_login',
      'LoginDetected': 'login_detected',
      'NavigatingToUpload': 'navigating',
      'ExtractingCredentials': 'extracting',
      'ClosingBrowser': 'closing',
      'Completed': 'completed',
      'Failed': 'error',
    }
    return stepMap[backendStep] || 'idle'
  }

  // Start authentication flow
  const startAuth = async (platform: string) => {
    currentPlatform.value = platform
    isAuthenticating.value = true
    state.value.error = null
    state.value.step = 'launching'
    state.value.message = '正在启动浏览器...'

    try {
      const result = await startBrowserAuth(platform)
      state.value.step = getAuthStep(result.step)
      state.value.message = result.message
      state.value.currentUrl = result.currentUrl || ''
      state.value.nickname = result.nickname
      state.value.avatarUrl = result.avatarUrl
      state.value.thirdId = result.thirdId
      state.value.secUid = result.secUid
      state.value.error = result.error || null

      console.log('[Auth] result.needPoll:', result.needPoll, 'result.step:', result.step)

      // Start polling if waiting for login
      if (result.needPoll) {
        console.log('[Auth] Starting polling...')
        startPolling()
      } else if (result.step === 'Completed') {
        isAuthenticating.value = false
      }
    } catch (error: any) {
      state.value.step = 'error'
      state.value.message = '启动授权失败'
      state.value.error = error.message || 'Unknown error'
      isAuthenticating.value = false
    }
  }

  // Start polling to check auth status
  const startPolling = () => {
    pollTimer = window.setInterval(async () => {
      if (!isAuthenticating.value) {
        stopPolling()
        return
      }

      try {
        const result = await checkBrowserAuthStatus()
        state.value.step = getAuthStep(result.step)
        state.value.message = result.message
        state.value.currentUrl = result.currentUrl
        state.value.screenshot = result.screenshot || state.value.screenshot
        state.value.nickname = result.nickname || state.value.nickname
        state.value.avatarUrl = result.avatarUrl || state.value.avatarUrl
        state.value.thirdId = result.thirdId || state.value.thirdId
        state.value.secUid = result.secUid || state.value.secUid
        state.value.error = result.error || null

        // If completed or no longer needs polling, stop
        if (!result.needPoll || result.step === 'Completed') {
          isAuthenticating.value = false
          stopPolling()
        }
      } catch (error: any) {
        // Continue polling on error
        console.error('[Auth] Polling error:', error)
      }
    }, 1000) // Poll every 1 second for faster detection
  }

  const stopPolling = () => {
    if (pollTimer) {
      clearInterval(pollTimer)
      pollTimer = null
    }
  }

  // Cancel authentication
  const cancelAuth = async () => {
    stopPolling()
    isAuthenticating.value = false

    try {
      await cancelBrowserAuth()
    } catch (error) {
      console.error('Error canceling auth:', error)
    }

    state.value.step = 'idle'
    state.value.message = '已取消授权'
    state.value.error = null
  }

  // Clean up on unmount
  onUnmounted(() => {
    stopPolling()
  })

  return {
    state,
    isAuthenticating,
    currentPlatform,
    startAuth,
    cancelAuth,
  }
}
