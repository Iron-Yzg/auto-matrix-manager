<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { PLATFORMS, type Platform } from '../types'
import {
  getPublicationAccountDetail,
  getCommentsByAwemeId,
  deleteComments,
  extractComments,
  getCommentCount,
  type Comment
} from '../services/api'

// Props from router query params (只接收 id，即 publication_accounts 表的主键)
const props = defineProps<{
  id?: string
  accountName?: string
}>()

const router = useRouter()
const route = useRoute()

// 解析路由参数：publication_accounts 表的主键 ID
const detailId = computed(() => props.id || route.query.id as string || '')
console.log('[CommentDetail] detailId:', detailId.value)

// Load account detail if id is provided
const accountDetail = ref<any>(null)
const loading = ref(true)
const awemeId = ref('') // 视频ID (item_id)

// Pagination state
const CURRENT_PAGE_SIZE = 50
const currentPage = ref(1)        // 显示的页码
const loadOffset = ref(0)         // 当前加载的offset位置
const hasMoreData = ref(true)     // 是否还有更多数据可加载（数据库中）
const showClearConfirm = ref(false) // 显示清空确认对话框
const isLoadingMore = ref(false)  // 是否正在加载更多
const isExtracting = ref(false)   // 是否正在提取评论
const extractProgress = ref('')   // 提取进度提示

// Refs for scroll detection
const commentsContainerRef = ref<HTMLElement | null>(null)

// Comments data
const comments = ref<Comment[]>([])
const totalComments = ref(0) // 数据库中评论总数

// 初始化：查询 publication_accounts 获取 item_id
onMounted(async () => {
  if (!detailId.value) {
    console.error('[CommentDetail] 未传入 detailId')
    loading.value = false
    return
  }

  try {
    const detail = await getPublicationAccountDetail(detailId.value) as any
    console.log('[CommentDetail] 获取到的详情:', JSON.stringify(detail, null, 2))

    // 兼容 itemId 和 item_id 两种格式
    const itemIdValue = detail?.itemId || detail?.item_id
    if (detail && itemIdValue) {
      accountDetail.value = detail
      awemeId.value = itemIdValue
      console.log('[CommentDetail] 获取到 awemeId:', awemeId.value)

      // 加载第一页评论
      await loadFirstPage()
    } else {
      console.error('[CommentDetail] 未找到 publication_accounts 或没有 itemId')
    }
  } catch (error) {
    console.error('[CommentDetail] 获取账号详情失败:', error)
  } finally {
    loading.value = false
  }
})

// 加载第一页评论
const loadFirstPage = async () => {
  if (!awemeId.value) return

  loadOffset.value = 0
  currentPage.value = 1
  await loadPage(0, true)

  // 初始化时从数据库获取正确的总数
  if (awemeId.value) {
    try {
      const dbCount = await getCommentCount(awemeId.value)
      totalComments.value = dbCount
      console.log(`[Comment] 初始化：从数据库获取总数=${dbCount}`)
    } catch (e) {
      console.error('[Comment] 获取数据库总数失败:', e)
    }
  }
}

// 加载指定offset的评论
const loadPage = async (offset: number, isFirstPage: boolean = false) => {
  if (!awemeId.value) return

  try {
    // 计算页码：offset / PAGE_SIZE + 1
    const page = Math.floor(offset / CURRENT_PAGE_SIZE) + 1
    const result = await getCommentsByAwemeId(awemeId.value, page, CURRENT_PAGE_SIZE)
    const pageComments = result.comments.map((c: any) => ({
      id: c.id || '',
      accountId: c.accountId || c.account_id || '',
      awemeId: c.awemeId || c.aweme_id || '',
      commentId: c.commentId || c.comment_id || '',
      userId: c.userId || c.user_id || '',
      userNickname: c.userNickname || c.user_nickname || '',
      userAvatar: c.userAvatar || c.user_avatar || '',
      content: c.content || '',
      likeCount: c.likeCount || c.like_count || 0,
      replyCount: c.replyCount || c.reply_count || 0,
      createTime: c.createTime || c.create_time || '',
      status: c.status || 'Pending',
      createdAt: c.createdAt || c.created_at || '',
    }))

    if (isFirstPage) {
      // 初始加载：替换数据
      comments.value = pageComments
      totalComments.value = result.total || 0
      hasMoreData.value = pageComments.length >= CURRENT_PAGE_SIZE
    } else if (pageComments.length > 0) {
      // 后续加载且有数据：追加（去重）
      const existingIds = new Set(comments.value.map(c => c.id))
      const newComments = pageComments.filter(c => !existingIds.has(c.id))
      if (newComments.length > 0) {
        comments.value = [...comments.value, ...newComments]
      }
      hasMoreData.value = pageComments.length >= CURRENT_PAGE_SIZE
    } else {
      // 没有数据了
      hasMoreData.value = false
    }

    console.log(`[Comment] 加载offset=${offset}，返回 ${pageComments.length} 条，hasMoreData=${hasMoreData.value}`)
  } catch (error) {
    console.error('[Comment] 加载评论失败:', error)
    // 失败时offset不变，下次重试
  }
}

// 带去重的加载（用于提取后刷新）
const loadPageWithDedup = async (offset: number) => {
  if (!awemeId.value) return

  try {
    const page = Math.floor(offset / CURRENT_PAGE_SIZE) + 1
    const result = await getCommentsByAwemeId(awemeId.value, page, CURRENT_PAGE_SIZE)
    const pageComments = result.comments.map((c: any) => ({
      id: c.id || '',
      accountId: c.accountId || c.account_id || '',
      awemeId: c.awemeId || c.aweme_id || '',
      commentId: c.commentId || c.comment_id || '',
      userId: c.userId || c.user_id || '',
      userNickname: c.userNickname || c.user_nickname || '',
      userAvatar: c.userAvatar || c.user_avatar || '',
      content: c.content || '',
      likeCount: c.likeCount || c.like_count || 0,
      replyCount: c.replyCount || c.reply_count || 0,
      createTime: c.createTime || c.create_time || '',
      status: c.status || 'Pending',
      createdAt: c.createdAt || c.created_at || '',
    }))

    // 去重合并
    const existingIds = new Set(comments.value.map(c => c.id))
    const newComments = pageComments.filter(c => !existingIds.has(c.id))

    if (newComments.length > 0) {
      comments.value = [...comments.value, ...newComments]
    }

    // 从数据库获取正确的总数
    const dbCount = await getCommentCount(awemeId.value)
    totalComments.value = dbCount
    hasMoreData.value = pageComments.length >= CURRENT_PAGE_SIZE

    console.log(`[Comment] 去重加载: offset=${offset}, 新增 ${newComments.length} 条，总数 ${totalComments.value}`)
  } catch (error) {
    console.error('[Comment] 去重加载失败:', error)
  }
}

// 加载更多（滚动到底部时调用）
const loadMore = async () => {
  if (!hasMoreData.value || isLoadingMore.value || isExtracting.value || !awemeId.value) return

  isLoadingMore.value = true
  try {
    // 记录当前的offset
    const currentOffset = loadOffset.value
    // 计算下一页的offset（当前offset + 每页大小）
    const nextOffset = currentOffset + CURRENT_PAGE_SIZE

    console.log(`[Comment] loadMore: 当前offset=${currentOffset}, 下页offset=${nextOffset}`)

    // 先递增offset，这样即使失败下次也会尝试下一页
    loadOffset.value = nextOffset
    currentPage.value = Math.floor(nextOffset / CURRENT_PAGE_SIZE) + 1

    await loadPage(nextOffset, false)
  } finally {
    isLoadingMore.value = false
  }
}

// 提取评论（每次提取50条）
const extractMoreComments = async () => {
  if (!detailId.value || !awemeId.value || isExtracting.value) return

  isExtracting.value = true
  extractProgress.value = '提取中...'

  try {
    // cursor = 当前数据库中的总数，即从第cursor条开始提取
    const cursor = totalComments.value
    const count = CURRENT_PAGE_SIZE

    console.log(`[Comment] 提取评论: cursor=${cursor}, count=${count}`)

    const result = await extractComments(detailId.value, awemeId.value, count, cursor)

    if (result.success) {
      // 确保 total_extracted 是数字（Tauri返回snake_case）
      const extracted = typeof result.total_extracted === 'number' ? result.total_extracted : 0
      extractProgress.value = `提取成功 +${extracted} 条`

      // 提取成功后，刷新当前页数据（需要去重，会从数据库获取正确的总数）
      await loadPageWithDedup(loadOffset.value)

      // 刷新账号详情（更新publication_accounts中的comments数量）
      try {
        const detail = await getPublicationAccountDetail(detailId.value) as any
        if (detail) {
          accountDetail.value = detail
        }
      } catch (e) {
        console.error('[CommentDetail] 刷新账号详情失败:', e)
      }

      // 提取成功后，允许继续分页（如果有更多数据）
      if (extracted >= count) {
        hasMoreData.value = true
      }

      console.log(`[Comment] 提取成功，新增 ${extracted} 条`)
    } else {
      extractProgress.value = `提取失败: ${result.error_message || '未知错误'}`
    }
  } catch (error: any) {
    extractProgress.value = `提取失败: ${error.message || '未知错误'}`
    console.error('[Comment] 提取评论失败:', error)
  } finally {
    isExtracting.value = false
    // 3秒后清除进度
    setTimeout(() => {
      extractProgress.value = ''
    }, 3000)
  }
}

// Handle scroll event
const handleScroll = async () => {
  const container = commentsContainerRef.value
  if (!container) return

  const { scrollTop, scrollHeight, clientHeight } = container
  const distanceToBottom = scrollHeight - scrollTop - clientHeight

  // 当滚动到距离底部50px以内时加载更多
  if (distanceToBottom < 50) {
    console.log('[Comment] 滚动到底部，距离:', distanceToBottom, '触发loadMore')
    await loadMore()
  }
}

const getPlatformInfo = (platform: Platform) => PLATFORMS.find(p => p.id === platform) || PLATFORMS[0]
const goBack = () => router.back()

// 清空并重新提取
const handleClearComments = async () => {
  console.log('[CommentDetail] handleClearComments 被点击')
  if (!awemeId.value) {
    console.error('[CommentDetail] awemeId 为空')
    return
  }
  showClearConfirm.value = true
}

const confirmClearComments = async () => {
  showClearConfirm.value = false
  console.log('[CommentDetail] 用户确认清空, awemeId:', awemeId.value)

  try {
    const result = await deleteComments(awemeId.value)
    console.log('[CommentDetail] 删除结果:', result)

    comments.value = []
    totalComments.value = 0
    currentPage.value = 1
    loadOffset.value = 0
    hasMoreData.value = true
  } catch (error: any) {
    console.error('[CommentDetail] 清空评论失败:', error)
    alert('清空失败: ' + (error.message || error))
  }
}

const cancelClearComments = () => {
  showClearConfirm.value = false
  console.log('[CommentDetail] 用户取消清空')
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
    <!-- Header with Back, Account, Search and Actions -->
    <div class="flex items-center justify-between gap-4 mb-4 flex-shrink-0">
      <div class="flex items-center gap-3 min-w-0">
        <button @click="goBack" class="p-2 hover:bg-slate-100 rounded-lg transition-colors flex-shrink-0">
          <svg class="w-5 h-5 text-slate-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" />
          </svg>
        </button>
        <div class="flex items-center gap-2 px-3 py-1.5 bg-white rounded-full border border-slate-200 min-w-0">
          <img v-if="accountDetail" :src="getPlatformInfo(accountDetail.platform?.toLowerCase() || 'douyin').icon" :alt="getPlatformInfo(accountDetail.platform?.toLowerCase() || 'douyin').name" class="w-4 h-4 object-contain flex-shrink-0" />
          <span class="text-sm text-slate-600 truncate">{{ accountDetail?.account_name || '评论详情' }}</span>
        </div>
        <!-- Search Input -->
        <div class="relative w-56">
          <svg class="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-slate-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
          </svg>
          <input v-model="searchQuery" type="text" placeholder="搜索评论..." class="w-full pl-9 pr-3 py-1.5 bg-white border border-slate-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-indigo-500/10" />
        </div>
        <!-- Sort Dropdown -->
        <select v-model="sortBy" class="px-3 py-1.5 bg-white border border-slate-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-indigo-500/10 cursor-pointer">
          <option value="time">按时间</option>
          <option value="likes">按热度</option>
        </select>
      </div>
      <div class="flex items-center gap-3 flex-shrink-0">
        <!-- Extracting progress -->
        <span v-if="extractProgress" class="text-sm text-amber-600 flex items-center gap-2">
          {{ extractProgress }}
        </span>
        <!-- Extract button -->
        <button
          @click="extractMoreComments"
          :disabled="isExtracting"
          class="px-3 py-1.5 bg-indigo-50 text-indigo-600 text-sm font-medium rounded-lg hover:bg-indigo-100 transition-colors flex items-center gap-1.5 disabled:opacity-50 disabled:cursor-not-allowed"
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
          </svg>
          提取评论
        </button>
        <!-- Clear button -->
        <button
          v-if="comments.length > 0"
          @click="handleClearComments"
          class="px-3 py-1.5 bg-rose-50 text-rose-600 text-sm font-medium rounded-lg hover:bg-rose-100 transition-colors flex items-center gap-1.5"
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
          </svg>
          清空
        </button>
        <!-- Count display: 当前加载数量/总数量 -->
        <span class="text-sm text-slate-500 flex-shrink-0">{{ comments.length }}/{{ totalComments }} 条</span>
      </div>
    </div>

    <!-- Loading State -->
    <div v-if="loading" class="flex-1 flex items-center justify-center">
      <div class="w-8 h-8 border-4 border-indigo-500 border-t-transparent rounded-full animate-spin"></div>
    </div>

    <!-- No Data State -->
    <div v-else-if="!awemeId" class="flex-1 flex items-center justify-center">
      <div class="text-center">
        <div class="w-14 h-14 mx-auto mb-3 rounded-full bg-slate-100 flex items-center justify-center">
          <svg class="w-7 h-7 text-slate-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
          </svg>
        </div>
        <h3 class="text-sm font-medium text-slate-600 mb-1">无法加载评论</h3>
        <p class="text-xs text-slate-400">未找到视频信息</p>
      </div>
    </div>

    <!-- Main Content -->
    <div v-else class="flex-1 flex flex-col">
      <!-- Comments List -->
      <div class="flex-1 bg-white rounded-2xl border border-slate-200 overflow-hidden flex flex-col max-h-[calc(100vh-100px)]">
        <div
          ref="commentsContainerRef"
          @scroll="handleScroll"
          class="overflow-y-auto flex-1 h-full"
        >
          <div class="divide-y divide-slate-100 min-h-full">
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

        <!-- Loading More Indicator -->
        <div v-if="isLoadingMore" class="py-3 text-center">
          <div class="inline-flex items-center gap-2 text-sm text-slate-500">
            <svg class="w-4 h-4 animate-spin" fill="none" viewBox="0 0 24 24">
              <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
              <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"></path>
            </svg>
            加载中...
          </div>
        </div>

        <!-- End of List -->
        <div v-else-if="!hasMoreData && comments.length > 0" class="py-3 text-center">
          <p class="text-xs text-slate-400">已加载全部 {{ totalComments }} 条评论</p>
        </div>

        <!-- Empty State -->
        <div v-else-if="filteredComments.length === 0" class="py-12 text-center">
          <div class="w-14 h-14 mx-auto mb-3 rounded-full bg-slate-100 flex items-center justify-center">
            <svg class="w-7 h-7 text-slate-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8a9.863-9 8 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z" />
            </svg>
          </div>
          <h3 class="text-sm font-medium text-slate-600 mb-1">暂无评论</h3>
          <p class="text-xs text-slate-400">数据库中暂无评论数据</p>
        </div>
      </div>
    </div>

    <!-- Custom Clear Confirmation Dialog -->
    <div v-if="showClearConfirm" class="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div class="bg-white rounded-xl p-6 max-w-sm mx-4 shadow-xl">
        <div class="w-12 h-12 mx-auto mb-4 rounded-full bg-rose-100 flex items-center justify-center">
          <svg class="w-6 h-6 text-rose-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
          </svg>
        </div>
        <h3 class="text-lg font-semibold text-slate-800 text-center mb-2">确认清空</h3>
        <p class="text-slate-600 text-center mb-6">确定要清空所有评论吗？此操作不可恢复。</p>
        <div class="flex gap-3">
          <button @click="cancelClearComments" class="flex-1 px-4 py-2.5 text-slate-600 font-medium bg-slate-100 rounded-xl hover:bg-slate-200 transition-colors">
            取消
          </button>
          <button @click="confirmClearComments" class="flex-1 px-4 py-2.5 text-white font-medium bg-rose-600 rounded-xl hover:bg-rose-700 transition-colors">
            确定
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
