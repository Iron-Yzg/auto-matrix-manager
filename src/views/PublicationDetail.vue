<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { PLATFORMS, type Platform, type PublishedAccount } from '../types'

const route = useRoute()
const router = useRouter()

const publicationId = route.params.id as string
console.log('Fetching publication:', publicationId)

const publication = ref({
  id: '1', title: '我的第一个视频', description: '这是一个测试视频', videoPath: '/videos/test-video.mp4', coverPath: '/covers/cover1.jpg', createdAt: '2024-01-15 10:30:00',
  publishedAccounts: [
    { id: 'p1', accountId: '1', platform: 'douyin' as Platform, accountName: '抖音测试号', status: 'success', publishUrl: 'https://douyin.com/video/123', publishTime: '2024-01-15 10:35:00', stats: { comments: 128, likes: 2560, favorites: 512, shares: 128 } },
    { id: 'p2', accountId: '2', platform: 'xiaohongshu' as Platform, accountName: '小红书运营号', status: 'success', publishUrl: 'https://xiaohongshu.com/post/456', publishTime: '2024-01-15 10:36:00', stats: { comments: 256, likes: 5120, favorites: 1024, shares: 256 } },
    { id: 'p3', accountId: '3', platform: 'kuaishou' as Platform, accountName: '快手创作号', status: 'publishing', stats: { comments: 0, likes: 0, favorites: 0, shares: 0 } },
    { id: 'p4', accountId: '4', platform: 'bilibili' as Platform, accountName: 'B站UP主', status: 'failed', stats: { comments: 0, likes: 0, favorites: 0, shares: 0 } }
  ] as PublishedAccount[]
})

const getPlatformInfo = (platform: Platform) => PLATFORMS.find(p => p.id === platform)!

const getStatusConfig = (status: string) => {
  switch (status) {
    case 'success': return { text: '发布成功', bg: 'bg-emerald-50', textColor: 'text-emerald-600', dot: 'bg-emerald-500' }
    case 'publishing': return { text: '发布中', bg: 'bg-amber-50', textColor: 'text-amber-600', dot: 'bg-amber-500' }
    case 'failed': return { text: '发布失败', bg: 'bg-rose-50', textColor: 'text-rose-600', dot: 'bg-rose-500' }
    default: return { text: '待发布', bg: 'bg-slate-100', textColor: 'text-slate-600', dot: 'bg-slate-500' }
  }
}

const formatNumber = (num: number) => {
  if (num >= 10000) return (num / 10000).toFixed(1) + 'w'
  if (num >= 1000) return (num / 1000).toFixed(1) + 'k'
  return num.toString()
}

const viewComments = (account: PublishedAccount) => {
  router.push({ path: '/comments', query: { publicationId: publication.value.id, accountId: account.accountId, platform: account.platform, accountName: account.accountName } })
}

const goBack = () => router.back()

onMounted(() => {
  const interval = setInterval(() => {
    publication.value.publishedAccounts.forEach(pa => {
      if (pa.status === 'success') {
        pa.stats.comments += Math.floor(Math.random() * 3)
        pa.stats.likes += Math.floor(Math.random() * 10)
        pa.stats.favorites += Math.floor(Math.random() * 5)
      }
    })
  }, 5000)
  return () => clearInterval(interval)
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
      <span class="text-sm text-slate-500 truncate">{{ publication.title }}</span>
    </div>

    <!-- Platform List -->
    <div class="flex-1 bg-white rounded-2xl border border-slate-200 overflow-hidden flex flex-col">
      <div class="overflow-y-auto flex-1">
        <table class="w-full">
          <thead class="bg-slate-50 border-b border-slate-200 sticky top-0">
            <tr>
              <th class="text-left px-4 py-3 text-xs font-semibold text-slate-500 uppercase tracking-wider">平台</th>
              <th class="text-left px-4 py-3 text-xs font-semibold text-slate-500 uppercase tracking-wider">状态</th>
              <th class="text-left px-4 py-3 text-xs font-semibold text-slate-500 uppercase tracking-wider">评论</th>
              <th class="text-left px-4 py-3 text-xs font-semibold text-slate-500 uppercase tracking-wider">点赞</th>
              <th class="text-left px-4 py-3 text-xs font-semibold text-slate-500 uppercase tracking-wider">收藏</th>
              <th class="text-left px-4 py-3 text-xs font-semibold text-slate-500 uppercase tracking-wider">分享</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-slate-100">
            <tr v-for="account in publication.publishedAccounts" :key="account.id" class="hover:bg-slate-50/80 transition-colors">
              <td class="px-4 py-3">
                <div class="flex items-center gap-3">
                  <div class="w-8 h-8 rounded-lg bg-slate-100 flex items-center justify-center overflow-hidden">
                    <img :src="getPlatformInfo(account.platform).icon" :alt="account.accountName" class="w-5 h-5 object-contain" />
                  </div>
                  <span class="text-sm font-medium text-slate-700">{{ account.accountName }}</span>
                </div>
              </td>
              <td class="px-4 py-3">
                <span :class="['inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium', getStatusConfig(account.status).bg, getStatusConfig(account.status).textColor]">
                  <span :class="['w-1.5 h-1.5 rounded-full', getStatusConfig(account.status).dot]"></span>
                  {{ getStatusConfig(account.status).text }}
                </span>
              </td>
              <td class="px-4 py-3">
                <button @click="viewComments(account)" class="flex items-center gap-1 px-2 py-1 rounded-lg hover:bg-slate-100 transition-colors">
                  <span class="text-sm font-medium text-slate-700">{{ formatNumber(account.stats.comments) }}</span>
                  <svg class="w-3.5 h-3.5 text-slate-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
                  </svg>
                </button>
              </td>
              <td class="px-4 py-3 text-sm text-slate-600">{{ formatNumber(account.stats.likes) }}</td>
              <td class="px-4 py-3 text-sm text-slate-600">{{ formatNumber(account.stats.favorites) }}</td>
              <td class="px-4 py-3 text-sm text-slate-600">{{ formatNumber(account.stats.shares) }}</td>
            </tr>
          </tbody>
        </table>
      </div>

      <!-- Progress Bar -->
      <div v-for="account in publication.publishedAccounts" :key="'prog-'+account.id">
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
