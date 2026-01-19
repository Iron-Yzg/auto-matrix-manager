<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { PLATFORMS } from '../types'
import { getPublicationTaskWithAccounts } from '../services/api'

const route = useRoute()
const router = useRouter()

const publicationId = route.params.id as string
const loading = ref(true)

const publication = ref<any>(null)
const publishedAccounts = ref<any[]>([])

const getPlatformInfo = (platform: string) => PLATFORMS.find(p => p.id === platform) || PLATFORMS[0]

const getStatusConfig = (status: string) => {
  switch (status) {
    case 'completed': case 'success': return { text: '发布成功', bg: 'bg-emerald-50', textColor: 'text-emerald-600', dot: 'bg-emerald-500' }
    case 'publishing': return { text: '发布中', bg: 'bg-amber-50', textColor: 'text-amber-600', dot: 'bg-amber-500' }
    case 'failed': return { text: '发布失败', bg: 'bg-rose-50', textColor: 'text-rose-600', dot: 'bg-rose-500' }
    default: return { text: '草稿', bg: 'bg-slate-100', textColor: 'text-slate-600', dot: 'bg-slate-500' }
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

// Open comments page - always navigate to CommentDetail.vue
const openComments = (account: any) => {
  console.log('[Debug] openComments called:', { accountId: account.id, detailId: account.id })
  // Navigate to CommentDetail page with the account detail ID
  router.push(`/comments/${account.id}`)
}

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
      // 直接使用子表的 accountName 字段
      publishedAccounts.value = task.accounts.map((acc: any) => ({
        id: acc.id,
        accountId: acc.account_id,
        accountName: acc.account_name,  // 后端返回的是 snake_case
        platform: acc.platform.toLowerCase(),
        status: acc.status.toLowerCase(),
        publishUrl: acc.publish_url,
        publishedAt: acc.published_at,
        stats: acc.stats
      }))
      // Debug log to check publishUrl
      console.log('[Debug] Published accounts:', publishedAccounts.value.map(a => ({
        id: a.id,
        accountName: a.accountName,
        publishUrl: a.publishUrl,
        status: a.status
      })))
    }
  } catch (error) {
    console.error('Failed to load publication detail:', error)
  } finally {
    loading.value = false
  }
})
</script>

<template>
  <div class="h-full flex flex-col p-6">
    <!-- Header with Back -->
    <div class="flex items-center gap-3 mb-4 flex-shrink-0">
      <button @click="goBack" class="p-2 hover:bg-slate-100 rounded-lg transition-colors">
        <svg class="w-5 h-5 text-slate-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" />
        </svg>
      </button>
      <span class="text-sm text-slate-500 truncate">{{ publication?.title || '加载中...' }}</span>
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
              <th class="text-left px-4 py-3 text-xs font-semibold text-slate-500 uppercase tracking-wider">状态</th>
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
                <span :class="['inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium', getStatusConfig(account.status).bg, getStatusConfig(account.status).textColor]">
                  <span :class="['w-1.5 h-1.5 rounded-full', getStatusConfig(account.status).dot]"></span>
                  {{ getStatusConfig(account.status).text }}
                </span>
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

      <!-- Progress Bar for Publishing -->
      <div v-for="account in publishedAccounts" :key="'prog-'+account.id">
        <div v-if="account.status === 'publishing'" class="px-4 py-3 border-t border-slate-100 bg-slate-50">
          <div class="flex items-center justify-between text-xs text-slate-500 mb-1.5">
            <span>正在发布...</span>
            <span class="font-medium">75%</span>
          </div>
          <div class="h-1.5 bg-slate-200 rounded-full overflow-hidden">
            <div class="h-full bg-indigo-600 rounded-full transition-all duration-500" style="width: 75%"></div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
