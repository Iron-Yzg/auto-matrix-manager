<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { PLATFORMS, type Platform } from '../types'
import { getPublicationAccountDetail } from '../services/api'

const route = useRoute()
const router = useRouter()

// Support both query params and route id param
const accountId = route.params.id as string || route.query.accountId as string
// Use accountDetail data if available, otherwise fall back to query params
const accountNameParam = route.query.accountName as string

// Load account detail if id is provided
const accountDetail = ref<any>(null)
const loading = ref(true)

onMounted(async () => {
  if (accountId) {
    try {
      const detail = await getPublicationAccountDetail(accountId)
      if (detail) {
        accountDetail.value = detail
      }
    } catch (error) {
      console.error('Failed to load account detail:', error)
    }
  }
  loading.value = false
})

// Mock comments data
const comments = ref([
  { id: '1', username: '用户_a7x9k2', avatar: 'https://api.dicebear.com/7.x/avataaars/svg?seed=user1', content: '这个视频太棒了！内容很有价值，学到了很多', time: '2小时前', likes: 128, replyCount: 12 },
  { id: '2', username: '用户_m3n5p8', avatar: 'https://api.dicebear.com/7.x/avataaars/svg?seed=user2', content: '支持下，期待下个视频', time: '3小时前', likes: 89, replyCount: 5 },
  { id: '3', username: '用户_k2j4h6', avatar: 'https://api.dicebear.com/7.x/avataaars/svg?seed=user3', content: '请问这个功能在哪里设置？我想学一下', time: '4小时前', likes: 45, replyCount: 8 },
  { id: '4', username: '用户_p8q2r5', avatar: 'https://api.dicebear.com/7.x/avataaars/svg?seed=user4', content: '收藏了，刚好需要这个教程', time: '5小时前', likes: 67, replyCount: 3 },
  { id: '5', username: '用户_w4t6y9', avatar: 'https://api.dicebear.com/7.x/avataaars/svg?seed=user5', content: '做得不错，继续加油！', time: '6小时前', likes: 34, replyCount: 0 },
  { id: '6', username: '用户_e1r3u7', avatar: 'https://api.dicebear.com/7.x/avataaars/svg?seed=user6', content: '博主好专业，讲解得很清楚', time: '8小时前', likes: 156, replyCount: 15 },
  { id: '7', username: '用户_o5i8a2', avatar: 'https://api.dicebear.com/7.x/avataaars/svg?seed=user7', content: '已三连，支持up主', time: '10小时前', likes: 203, replyCount: 7 }
])

const searchQuery = ref('')
const sortBy = ref<'time' | 'likes'>('time')

const getPlatformInfo = (platform: Platform) => PLATFORMS.find(p => p.id === platform)!
const goBack = () => router.back()

const formatNumber = (num: number) => {
  if (num >= 10000) return (num / 10000).toFixed(1) + 'w'
  if (num >= 1000) return (num / 1000).toFixed(1) + 'k'
  return num.toString()
}

const filteredComments = computed(() => {
  let result = [...comments.value]
  if (searchQuery.value) {
    const query = searchQuery.value.toLowerCase()
    result = result.filter(c => c.username.toLowerCase().includes(query) || c.content.toLowerCase().includes(query))
  }
  if (sortBy.value === 'likes') result.sort((a, b) => b.likes - a.likes)
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
      <span class="text-sm text-slate-500 flex-shrink-0">共 {{ comments.length }} 条评论</span>
    </div>

    <!-- Filters -->
    <div class="flex items-center gap-3 mb-4 flex-shrink-0">
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
              <img :src="comment.avatar" :alt="comment.username" class="w-10 h-10 rounded-full bg-slate-100 flex-shrink-0" />
              <div class="flex-1 min-w-0">
                <div class="flex items-center justify-between gap-2">
                  <div class="flex items-center gap-2 min-w-0">
                    <span class="text-sm font-medium text-slate-700 truncate">{{ comment.username }}</span>
                    <span class="text-xs text-slate-400 flex-shrink-0">{{ comment.time }}</span>
                  </div>
                  <div class="flex items-center gap-3 flex-shrink-0">
                    <button class="flex items-center gap-1 text-slate-500 hover:text-rose-500 transition-colors">
                      <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4.318 6.318a4.5 4.5 0 000 6.364L12 20.364l7.682-7.682a4.5 4.5 0 00-6.364-6.364L12 7.636l-1.318-1.318a4.5 4.5 0 00-6.364 0z" />
                      </svg>
                      <span class="text-xs">{{ formatNumber(comment.likes) }}</span>
                    </button>
                    <button class="flex items-center gap-1 text-slate-500 hover:text-blue-600 transition-colors">
                      <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z" />
                      </svg>
                      <span class="text-xs">{{ comment.replyCount }}</span>
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
        <p class="text-xs text-slate-400">还没有收到评论</p>
      </div>
    </div>
  </div>
</template>
