<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { PLATFORMS, type Platform } from '../types'
import {
  getPublicationAccountDetail,
  extractComments,
  getCommentsByAwemeId,
  deleteComments,
  type Comment
} from '../services/api'

// Props from router query params
const props = defineProps<{
  id?: string
  awemeId?: string
  accountName?: string
}>()

const router = useRouter()

// Support both props and fallback to empty
const accountId = props.id || ''
const videoAwemeId = props.awemeId || ''
const accountNameParam = props.accountName || ''

// Load account detail if id is provided
const accountDetail = ref<any>(null)
const loading = ref(true)
const extracting = ref(false)
const extractProgress = ref('')

// Comments data
const comments = ref<Comment[]>([])
const totalComments = ref(0)

onMounted(async () => {
  if (accountId) {
    try {
      const detail = await getPublicationAccountDetail(accountId)
      if (detail) {
        accountDetail.value = detail
        // Try to get aweme_id from account detail if not in query params
        if (!videoAwemeId && detail.itemId) {
          await loadComments(detail.itemId)
          // 自动提取评论
          await handleExtractComments(detail.itemId)
        }
      }
    } catch (error) {
      console.error('Failed to load account detail:', error)
    }
  }

  // If we have aweme_id, load comments and auto extract
  if (videoAwemeId) {
    await loadComments(videoAwemeId)
    // 自动提取评论
    await handleExtractComments(videoAwemeId)
  }

  loading.value = false
})

// Load comments for a video
const loadComments = async (videoId: string) => {
  try {
    const data = await getCommentsByAwemeId(videoId)
    comments.value = data.map((c: any) => ({
      id: c.id,
      accountId: c.accountId || c.account_id || '',
      awemeId: c.awemeId || c.aweme_id || '',
      commentId: c.commentId || c.comment_id || '',
      userId: c.userId || c.user_id || '',
      userNickname: c.userNickname || c.user_nickname || '',
      userAvatar: c.userAvatar || c.user_avatar || '',
      content: c.content,
      likeCount: c.likeCount || c.like_count || 0,
      replyCount: c.replyCount || c.reply_count || 0,
      createTime: c.createTime || c.create_time || '',
      status: c.status,
      createdAt: c.createdAt || c.created_at || '',
    }))
    totalComments.value = data.length
  } catch (error) {
    console.error('Failed to load comments:', error)
  }
}

const getPlatformInfo = (platform: Platform) => PLATFORMS.find(p => p.id === platform) || PLATFORMS[0]
const goBack = () => router.back()

// Extract comments from the video
const handleExtractComments = async (videoId?: string) => {
  const targetAwemeId = videoId || videoAwemeId
  if (!accountId || !targetAwemeId || extracting.value) return

  extracting.value = true
  extractProgress.value = '开始提取评论...'

  try {
    const result = await extractComments(accountId, targetAwemeId, 500)

    if (result.success) {
      extractProgress.value = `提取完成！共 ${result.totalExtracted} 条评论`

      // Reload comments
      await loadComments(targetAwemeId)
    } else {
      extractProgress.value = `提取失败: ${result.errorMessage || '未知错误'}`
    }
  } catch (error: any) {
    extractProgress.value = `提取失败: ${error.message || '未知错误'}`
    console.error('Failed to extract comments:', error)
  } finally {
    extracting.value = false

    // Clear progress message after 3 seconds
    setTimeout(() => {
      extractProgress.value = ''
    }, 3000)
  }
}

// Delete all comments for the current video
const handleDeleteComments = async () => {
  if (!videoAwemeId || !confirm('确定要删除所有评论吗？此操作不可恢复。')) return

  try {
    await deleteComments(videoAwemeId)
    comments.value = []
    totalComments.value = 0
  } catch (error) {
    console.error('Failed to delete comments:', error)
  }
}

const formatNumber = (num: number) => {
  if (num >= 10000) return (num / 10000).toFixed(1) + 'w'
  if (num >= 1000) return (num / 1000).toFixed(1) + 'k'
  return num.toString()
}

const formatTime = (time: string | null | undefined) => {
  if (!time) return ''
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

const searchQuery = ref('')
const sortBy = ref<'time' | 'likes'>('time')

const filteredComments = computed(() => {
  let result = [...comments.value]
  if (searchQuery.value) {
    const query = searchQuery.value.toLowerCase()
    result = result.filter(c =>
      (c.userNickname?.toLowerCase().includes(query) || false) ||
      c.content.toLowerCase().includes(query)
    )
  }
  if (sortBy.value === 'likes') result.sort((a, b) => b.likeCount - a.likeCount)
  return result
})
</script>

<template>
  <div class="h-full flex flex-col p-6">
    <!-- Header with Back -->
    <div class="flex items-center justify-between gap-4 mb-4 flex-shrink-0">
      <div class="flex items-center gap-3 min-w-0">
        <button @click="goBack" class="p-2 hover:bg-slate-100 rounded-lg transition-colors flex-shrink-0">
          <svg class="w-5 h-5 text-slate-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" />
          </svg>
        </button>
        <div class="flex items-center gap-2 px-3 py-1.5 bg-white rounded-full border border-slate-200 min-w-0">
          <img v-if="accountDetail" :src="getPlatformInfo(accountDetail.platform?.toLowerCase() || 'douyin').icon" :alt="getPlatformInfo(accountDetail.platform?.toLowerCase() || 'douyin').name" class="w-4 h-4 object-contain flex-shrink-0" />
          <span class="text-sm text-slate-600 truncate">{{ accountDetail?.account_name || accountNameParam || '评论详情' }}</span>
        </div>
      </div>
      <div class="flex items-center gap-3 flex-shrink-0">
        <!-- Extract button -->
        <button
          v-if="videoAwemeId && !extracting"
          @click="() => handleExtractComments()"
          class="px-4 py-2 bg-indigo-600 text-white text-sm font-medium rounded-lg hover:bg-indigo-700 transition-colors flex items-center gap-2"
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
          </svg>
          提取评论
        </button>
        <!-- Extracting state -->
        <span v-if="extracting" class="text-sm text-amber-600 flex items-center gap-2">
          <svg class="w-4 h-4 animate-spin" fill="none" viewBox="0 0 24 24">
            <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
            <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"></path>
          </svg>
          {{ extractProgress || '提取中...' }}
        </span>
        <!-- Delete button -->
        <button
          v-if="comments.length > 0"
          @click="handleDeleteComments"
          class="px-4 py-2 bg-rose-50 text-rose-600 text-sm font-medium rounded-lg hover:bg-rose-100 transition-colors flex items-center gap-2"
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
          </svg>
          清空
        </button>
        <span class="text-sm text-slate-500 flex-shrink-0">共 {{ totalComments }} 条评论</span>
      </div>
    </div>

    <!-- Loading State -->
    <div v-if="loading" class="flex-1 flex items-center justify-center">
      <div class="w-8 h-8 border-4 border-indigo-500 border-t-transparent rounded-full animate-spin"></div>
    </div>

    <!-- Main Content -->
    <div v-else class="flex-1 flex flex-col gap-4">
      <!-- Filters -->
      <div class="flex items-center gap-3 flex-shrink-0">
        <div class="relative flex-1 max-w-xs">
          <svg class="w-5 h-5 absolute left-3 top-1/2 -translate-y-1/2 text-slate-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
          </svg>
          <input v-model="searchQuery" type="text" placeholder="搜索评论..." class="w-full pl-10 pr-4 py-2 bg-white border border-slate-200 rounded-xl focus:outline-none focus:ring-2 focus:ring-indigo-500/10" />
        </div>
        <select v-model="sortBy" class="px-4 py-2 bg-white border border-slate-200 rounded-xl focus:outline-none focus:ring-2 focus:ring-indigo-500/10 cursor-pointer">
          <option value="time">按时间</option>
          <option value="likes">按热度</option>
        </select>
      </div>

      <!-- Comments List -->
      <div class="flex-1 bg-white rounded-2xl border border-slate-200 overflow-hidden flex flex-col">
        <div class="overflow-y-auto flex-1">
          <div class="divide-y divide-slate-100">
            <div v-for="comment in filteredComments" :key="comment.id" class="p-4 hover:bg-slate-50/80 transition-colors">
              <div class="flex gap-3">
                <img :src="comment.userAvatar" :alt="comment.userNickname" class="w-10 h-10 rounded-full bg-slate-100 flex-shrink-0" onerror="this.src='https://api.dicebear.com/7.x/avataaars/svg?seed=default'" />
                <div class="flex-1 min-w-0">
                  <div class="flex items-center justify-between gap-2">
                    <div class="flex items-center gap-2 min-w-0">
                      <span class="text-sm font-medium text-slate-700 truncate">{{ comment.userNickname }}</span>
                      <span class="text-xs text-slate-400 flex-shrink-0">{{ formatTime(comment.createTime) }}</span>
                    </div>
                    <div class="flex items-center gap-3 flex-shrink-0">
                      <button class="flex items-center gap-1 text-slate-500 hover:text-rose-500 transition-colors">
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4.318 6.318a4.5 4.5 0 000 6.364L12 20.364l7.682-7.682a4.5 4.5 0 00-6.364-6.364L12 7.636l-1.318-1.318a4.5 4.5 0 00-6.364 0z" />
                        </svg>
                        <span class="text-xs">{{ formatNumber(comment.likeCount) }}</span>
                      </button>
                      <button class="flex items-center gap-1 text-slate-500 hover:text-blue-600 transition-colors">
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z" />
                        </svg>
                        <span class="text-xs">{{ formatNumber(comment.replyCount) }}</span>
                      </button>
                    </div>
                  </div>
                  <p class="text-sm text-slate-600 mt-1.5 leading-relaxed">{{ comment.content }}</p>
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- Empty State -->
        <div v-if="filteredComments.length === 0" class="py-12 text-center">
          <div class="w-14 h-14 mx-auto mb-3 rounded-full bg-slate-100 flex items-center justify-center">
            <svg class="w-7 h-7 text-slate-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z" />
            </svg>
          </div>
          <h3 class="text-sm font-medium text-slate-600 mb-1">暂无评论</h3>
          <p v-if="!videoAwemeId" class="text-xs text-slate-400">请先设置视频ID以加载评论</p>
          <p v-else class="text-xs text-slate-400">点击"提取评论"从抖音获取评论数据</p>
        </div>
      </div>
    </div>
  </div>
</template>
