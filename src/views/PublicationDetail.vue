<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { listen } from '@tauri-apps/api/event'
import { PLATFORMS } from '../types'
import { getPublicationTaskWithAccounts, publishPublicationTask, retryPublicationTask } from '../services/api'

// 进度事件类型
interface ProgressEvent {
  detail_id: string
  account_id: string
  platform: string
  status: string
  message: string
  progress: number
  timestamp: number
}

const route = useRoute()
const router = useRouter()

const publicationId = route.params.id as string
const loading = ref(true)
const publishing = ref(false)

const publication = ref<any>(null)
const publishedAccounts = ref<any[]>([])
// 存储每个账号的实时进度
const accountProgress = ref<Record<string, { progress: number; message: string; status: string }>>({})

const getPlatformInfo = (platform: string) => PLATFORMS.find(p => p.id === platform) || PLATFORMS[0]

// 获取账号的进度信息
const getAccountProgress = (accountId: string) => {
  return accountProgress.value[accountId] || null
}

// 判断账号是否正在发布中（有实时进度且未完成）
const isPublishing = (account: any) => {
  const progress = getAccountProgress(account.id)
  if (progress) {
    return progress.status !== 'completed' && progress.status !== 'failed'
  }
  return account.status === 'publishing'
}

const getStatusConfig = (status: string) => {
  switch (status) {
    case 'completed': case 'success': return { text: '发布成功', bg: 'bg-emerald-50', textColor: 'text-emerald-600', dot: 'bg-emerald-500', progress: 100, progressColor: 'bg-emerald-500' }
    case 'publishing': return { text: '发布中', bg: 'bg-amber-50', textColor: 'text-amber-600', dot: 'bg-amber-500', progress: null, progressColor: 'bg-amber-500' }
    case 'failed': return { text: '发布失败', bg: 'bg-rose-50', textColor: 'text-rose-600', dot: 'bg-rose-500', progress: 0, progressColor: 'bg-rose-400' }
    default: return { text: '草稿', bg: 'bg-slate-100', textColor: 'text-slate-600', dot: 'bg-slate-400', progress: 0, progressColor: 'bg-slate-300' }
  }
}

const formatNumber = (num: number) => {
  if (num >= 10000) return (num / 10000).toFixed(1) + 'w'
  if (num >= 1000) return (num / 1000).toFixed(1) + 'k'
  return num.toString()
}

const formatTime = (time: string | null) => {
  if (!time) return '-'
  try {
    const date = new Date(time)
    return date.toLocaleString('zh-CN', {
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit'
    })
  } catch {
    return time
  }
}

const goBack = () => router.back()

// Open comments page
const openComments = (account: any) => {
  router.push(`/comments/${account.id}`)
}

// 发布/重发
const handlePublish = async () => {
  if (publishing.value) return

  publishing.value = true
  // 清空进度
  accountProgress.value = {}

  try {
    // 判断是否有需要重发的账号
    const needsRetry = publishedAccounts.value.some(
      (acc: any) => acc.status === 'draft' || acc.status === 'failed'
    )

    let result
    if (needsRetry) {
      // 重发失败的或未发布的账号
      result = await retryPublicationTask(publicationId)
    } else {
      // 首次发布
      result = await publishPublicationTask(
        publicationId,
        publication.value?.title || '',
        publication.value?.description || '',
        publication.value?.videoPath || '',
        []
      )
    }

    if (result && result.results) {
      // 更新账号状态
      for (const r of result.results) {
        const account = publishedAccounts.value.find((acc: any) => acc.id === r.detailId)
        if (account) {
          account.status = r.success ? 'completed' : 'failed'
          account.publishUrl = r.publishUrl
          account.message = r.error || null
        }
      }

      // 更新主表状态
      const task = await getPublicationTaskWithAccounts(publicationId)
      if (task) {
        publication.value.status = task.status
      }
    }
  } catch (error) {
    console.error('发布失败:', error)
  } finally {
    publishing.value = false
  }
}

// 检查是否需要显示发布按钮
const canPublish = computed(() => {
  if (!publication.value) return false
  // 主表不是 Completed 状态，且有未发布或失败的账号
  if (publication.value.status === 'completed') return false
  return publishedAccounts.value.some(
    (acc: any) => acc.status === 'draft' || acc.status === 'failed'
  )
})

// 获取发布按钮文本
const publishButtonText = computed(() => {
  if (publishing.value) return '发布中...'
  const hasDraft = publishedAccounts.value.some((acc: any) => acc.status === 'draft')
  const hasFailed = publishedAccounts.value.some((acc: any) => acc.status === 'failed')
  if (hasFailed) return '重发失败账号'
  if (hasDraft) return '开始发布'
  return '发布'
})

// 监听进度事件
let unlistenProgress: (() => void) | null = null

onMounted(async () => {
  try {
    const task = await getPublicationTaskWithAccounts(publicationId)
    if (task) {
      publication.value = {
        id: task.id,
        title: task.title,
        description: task.description,
        videoPath: task.videoPath,
        coverPath: task.coverPath,
        createdAt: task.createdAt,
        publishedAt: task.publishedAt,
        status: task.status
      }
      publishedAccounts.value = task.accounts.map((acc: any) => ({
        id: acc.id,
        accountId: acc.account_id,
        accountName: acc.account_name,
        platform: acc.platform.toLowerCase(),
        status: acc.status.toLowerCase(),
        publishUrl: acc.publish_url,
        publishedAt: acc.published_at,
        stats: acc.stats,
        message: acc.message || null,
        itemId: acc.item_id || null
      }))
    }

    // 监听进度事件
    unlistenProgress = await listen<ProgressEvent>('publish-progress', (event) => {
      const progress = event.payload
      console.log('[Progress] Received progress event:', progress)
      // 更新对应账号的进度
      accountProgress.value[progress.detail_id] = {
        progress: progress.progress,
        message: progress.message,
        status: progress.status
      }
    })
  } catch (error) {
    console.error('Failed to load publication detail:', error)
  } finally {
    loading.value = false
  }
})

onUnmounted(() => {
  // 清理监听器
  if (unlistenProgress) {
    unlistenProgress()
  }
})
</script>

<template>
  <div class="h-full flex flex-col p-6">
    <!-- Header with Back -->
    <div class="flex items-center justify-between mb-4 flex-shrink-0">
      <div class="flex items-center gap-3">
        <button @click="goBack" class="p-2 hover:bg-slate-100 rounded-lg transition-colors">
          <svg class="w-5 h-5 text-slate-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" />
          </svg>
        </button>
        <span class="text-sm text-slate-500 truncate">{{ publication?.title || '加载中...' }}</span>
      </div>
      <!-- Publish/Retry Button -->
      <button
        v-if="canPublish && !publishing"
        @click="handlePublish"
        class="px-4 py-2 bg-indigo-600 text-white text-sm font-medium rounded-lg hover:bg-indigo-700 transition-colors flex items-center gap-2"
      >
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664z" />
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
        </svg>
        {{ publishButtonText }}
      </button>
      <span v-if="publishing" class="text-sm text-slate-500 flex items-center gap-2">
        <svg class="w-4 h-4 animate-spin" fill="none" viewBox="0 0 24 24">
          <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
          <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
        </svg>
        发布中...
      </span>
    </div>

    <!-- Loading State -->
    <div v-if="loading" class="flex-1 flex items-center justify-center">
      <div class="w-8 h-8 border-4 border-indigo-500 border-t-transparent rounded-full animate-spin"></div>
    </div>

    <!-- Platform List -->
    <div v-else class="flex-1 bg-white rounded-2xl border border-slate-200 overflow-hidden flex flex-col">
      <div class="overflow-y-auto flex-1">
        <table class="w-full">
          <thead class="bg-slate-50 border-b border-slate-200 sticky top-0">
            <tr>
              <th class="text-left px-4 py-3 text-xs font-semibold text-slate-500 uppercase tracking-wider">平台</th>
              <th class="text-left px-4 py-3 text-xs font-semibold text-slate-500 uppercase tracking-wider w-48">状态</th>
              <th class="text-left px-4 py-3 text-xs font-semibold text-slate-500 uppercase tracking-wider">发布时间</th>
              <th class="text-left px-4 py-3 text-xs font-semibold text-slate-500 uppercase tracking-wider">评论</th>
              <th class="text-left px-4 py-3 text-xs font-semibold text-slate-500 uppercase tracking-wider">点赞</th>
              <th class="text-left px-4 py-3 text-xs font-semibold text-slate-500 uppercase tracking-wider">收藏</th>
              <th class="text-left px-4 py-3 text-xs font-semibold text-slate-500 uppercase tracking-wider">分享</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-slate-100">
            <tr v-for="account in publishedAccounts" :key="account.id" class="hover:bg-slate-50/80 transition-colors">
              <td class="px-4 py-3">
                <div class="flex items-center gap-3">
                  <div class="w-8 h-8 rounded-lg bg-slate-100 flex items-center justify-center overflow-hidden">
                    <img :src="getPlatformInfo(account.platform).icon" :alt="account.platform" class="w-5 h-5 object-contain" />
                  </div>
                  <div class="min-w-0">
                    <span class="text-sm font-medium text-slate-700 capitalize">{{ account.accountName || '未知账号' }}</span>
                    <span class="text-xs text-slate-400 block">{{ getPlatformInfo(account.platform).name }}</span>
                  </div>
                </div>
              </td>
              <td class="px-4 py-3">
                <div v-if="isPublishing(account)" class="relative">
                  <!-- 发布中：显示进度条 -->
                  <div class="w-full">
                    <div class="flex items-center justify-between text-xs text-amber-600 mb-1">
                      <span class="flex items-center gap-1">
                        <svg class="w-3 h-3 animate-spin" fill="none" viewBox="0 0 24 24">
                          <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                          <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"></path>
                        </svg>
                        {{ getAccountProgress(account.id)?.message || '发布中...' }}
                      </span>
                      <span class="text-amber-600">{{ getAccountProgress(account.id)?.progress || 0 }}%</span>
                    </div>
                    <div class="h-1.5 bg-amber-100 rounded-full overflow-hidden">
                      <div
                        class="h-full bg-amber-500 rounded-full transition-all duration-300"
                        :style="{ width: (getAccountProgress(account.id)?.progress || 0) + '%' }"
                      ></div>
                    </div>
                  </div>
                </div>
                <div v-else class="relative group">
                  <!-- 非发布中：显示状态标签 -->
                  <span :class="['inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium cursor-help', getStatusConfig(account.status).bg, getStatusConfig(account.status).textColor]">
                    <span :class="['w-1.5 h-1.5 rounded-full', getStatusConfig(account.status).dot]"></span>
                    {{ getStatusConfig(account.status).text }}
                  </span>
                  <!-- Error tooltip -->
                  <div v-if="account.status === 'failed' && account.message" class="absolute left-0 top-full mt-1 px-3 py-2 bg-slate-800 text-white text-xs rounded-lg opacity-0 invisible group-hover:opacity-100 group-hover:visible transition-all z-50 max-w-xs whitespace-pre-wrap">
                    {{ account.message }}
                  </div>
                </div>
              </td>
              <td class="px-4 py-3 text-sm text-slate-500">{{ formatTime(account.publishedAt) }}</td>
              <td class="px-4 py-3 text-sm text-slate-600">
                <button
                  @click="openComments(account)"
                  class="text-indigo-600 hover:text-indigo-800 underline"
                >
                  {{ formatNumber(account.stats?.comments || 0) }}
                </button>
              </td>
              <td class="px-4 py-3 text-sm text-slate-600">{{ formatNumber(account.stats?.likes || 0) }}</td>
              <td class="px-4 py-3 text-sm text-slate-600">{{ formatNumber(account.stats?.favorites || 0) }}</td>
              <td class="px-4 py-3 text-sm text-slate-600">{{ formatNumber(account.stats?.shares || 0) }}</td>
            </tr>
          </tbody>
        </table>

        <!-- Empty State -->
        <div v-if="publishedAccounts.length === 0" class="py-12 text-center">
          <div class="w-16 h-16 mx-auto mb-3 rounded-full bg-slate-100 flex items-center justify-center">
            <svg class="w-8 h-8 text-slate-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0zm6 3a2 2 0 11-4 0 2 2 0 014 0zM7 10a2 2 0 11-4 0 2 2 0 014 0z" />
            </svg>
          </div>
          <h3 class="text-sm font-medium text-slate-600 mb-1">暂无发布账号</h3>
          <p class="text-xs text-slate-400">请先发布作品以添加账号</p>
        </div>
      </div>
    </div>
  </div>
</template>
