<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { PLATFORMS, type Platform } from '../types'
import { getAccounts, toFrontendAccount, getPublicationTasks, createPublicationTask, deletePublicationTask } from '../services/api'
import PublishDialog from '../components/PublishDialog.vue'

const router = useRouter()

const accounts = ref<Array<{
  id: string
  platform: string
  accountName: string
  avatar: string
  status: 'active' | 'expired' | 'pending'
  authorizedAt: string
}>>([])

const publications = ref<any[]>([])
const loading = ref(false)

const searchQuery = ref('')
const platformFilter = ref<Platform | 'all'>('all')
const statusFilter = ref<string>('all')

const showPublishDialog = ref(false)

// Open publish dialog
const openPublishDialog = () => {
  showPublishDialog.value = true
}

// File selection state removed - now handled in PublishDialog

const loadAccounts = async () => {
  try {
    const backendAccounts = await getAccounts('douyin')
    accounts.value = backendAccounts.map(toFrontendAccount)
  } catch (error) {
    console.error('Failed to load accounts:', error)
    accounts.value = []
  }
}

const loadPublications = async () => {
  loading.value = true
  try {
    const tasks = await getPublicationTasks()
    // Convert to frontend format
    publications.value = tasks.map(task => ({
      id: task.id,
      title: task.title,
      description: task.description,
      videoPath: task.videoPath,
      coverPath: task.coverPath,
      status: task.status.toLowerCase(),
      createdAt: task.createdAt,
      publishedAt: task.publishedAt,
      publishedAccounts: task.accounts.map(acc => ({
        id: acc.id,
        accountId: acc.accountId,
        platform: acc.platform.toLowerCase(),
        accountName: '',
        status: acc.status.toLowerCase(),
        publishUrl: acc.publishUrl,
        stats: acc.stats
      }))
    }))
  } catch (error) {
    console.error('Failed to load publications:', error)
    publications.value = []
  } finally {
    loading.value = false
  }
}

onMounted(() => {
  loadAccounts()
  loadPublications()
})

const filteredPublications = computed(() => {
  return publications.value.filter(pub => {
    const matchesSearch = pub.title?.toLowerCase().includes(searchQuery.value.toLowerCase()) ||
                          pub.description?.toLowerCase().includes(searchQuery.value.toLowerCase())
    const matchesPlatform = platformFilter.value === 'all' || pub.platform === platformFilter.value
    const matchesStatus = statusFilter.value === 'all' || pub.status === statusFilter.value
    return matchesSearch && matchesPlatform && matchesStatus
  })
})

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

const viewDetail = (id: string) => router.push(`/publications/${id}`)

const handleDelete = async (id: string) => {
  try {
    await deletePublicationTask(id)
    const index = publications.value.findIndex(p => p.id === id)
    if (index > -1) publications.value.splice(index, 1)
  } catch (error) {
    console.error('Failed to delete publication:', error)
  }
}

// Publish dialog handlers
const handlePublish = async (data: {
  title: string
  description: string
  videoPath: string
  coverPath: string | null
  accountIds: string[]
  platforms: string[]
  hashtags: string[][]
}) => {
  try {
    const task = await createPublicationTask(
      data.title,
      data.description,
      data.videoPath,
      data.coverPath,
      data.accountIds,
      data.platforms,
      data.hashtags
    )

    const newPublication = {
      id: task.id,
      title: task.title,
      description: task.description,
      videoPath: task.videoPath,
      coverPath: task.coverPath,
      status: task.status.toLowerCase(),
      createdAt: task.createdAt,
      publishedAt: task.publishedAt,
      publishedAccounts: task.accounts.map(acc => ({
        id: acc.id,
        accountId: acc.accountId,
        platform: acc.platform.toLowerCase(),
        accountName: '',
        status: acc.status.toLowerCase(),
        publishUrl: acc.publishUrl,
        stats: acc.stats
      }))
    }
    publications.value.unshift(newPublication)
    showPublishDialog.value = false
  } catch (error) {
    console.error('Failed to publish:', error)
    alert('发布失败: ' + error)
  }
}

</script>

<template>
  <div class="h-full flex flex-col p-6">
    <!-- Header -->
    <div class="flex items-center justify-between mb-4 flex-shrink-0">
      <div class="flex gap-3">
        <div class="relative">
          <svg class="w-5 h-5 absolute left-3 top-1/2 -translate-y-1/2 text-slate-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
          </svg>
          <input v-model="searchQuery" type="text" placeholder="搜索作品..." class="pl-10 pr-4 py-2.5 w-64 bg-white border border-slate-200 rounded-xl focus:outline-none focus:ring-2 focus:ring-indigo-500/10" />
        </div>
        <select v-model="platformFilter" class="px-4 py-2.5 bg-white border border-slate-200 rounded-xl focus:outline-none focus:ring-2 focus:ring-indigo-500/10 cursor-pointer">
          <option value="all">全部平台</option>
          <option v-for="p in PLATFORMS" :key="p.id" :value="p.id">{{ p.name }}</option>
        </select>
        <select v-model="statusFilter" class="px-4 py-2.5 bg-white border border-slate-200 rounded-xl focus:outline-none focus:ring-2 focus:ring-indigo-500/10 cursor-pointer">
          <option value="all">全部状态</option>
          <option value="draft">草稿</option>
          <option value="publishing">发布中</option>
          <option value="completed">已完成</option>
        </select>
      </div>
      <button @click="openPublishDialog" class="flex items-center gap-2 px-4 py-2.5 bg-indigo-600 text-white rounded-xl hover:bg-indigo-700 transition-all">
        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
        </svg>
        <span class="font-medium">发布作品</span>
      </button>
    </div>

    <!-- Publication List -->
    <div class="flex-1 bg-white rounded-2xl border border-slate-200 overflow-hidden flex flex-col">
      <div v-if="loading" class="flex-1 flex items-center justify-center">
        <div class="w-8 h-8 border-4 border-indigo-500 border-t-transparent rounded-full animate-spin"></div>
      </div>
      <div v-else class="overflow-y-auto flex-1">
        <table class="w-full">
          <thead class="bg-slate-50 border-b border-slate-200 sticky top-0">
            <tr>
              <th class="text-left px-6 py-3 text-xs font-semibold text-slate-500 uppercase tracking-wider">作品</th>
              <th class="text-left px-6 py-3 text-xs font-semibold text-slate-500 uppercase tracking-wider">发布平台</th>
              <th class="text-left px-6 py-3 text-xs font-semibold text-slate-500 uppercase tracking-wider">状态</th>
              <th class="text-left px-6 py-3 text-xs font-semibold text-slate-500 uppercase tracking-wider">评论</th>
              <th class="text-left px-6 py-3 text-xs font-semibold text-slate-500 uppercase tracking-wider">点赞</th>
              <th class="text-left px-6 py-3 text-xs font-semibold text-slate-500 uppercase tracking-wider">创建时间</th>
              <th class="text-right px-6 py-3 text-xs font-semibold text-slate-500 uppercase tracking-wider">操作</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-slate-100">
            <tr v-for="pub in filteredPublications" :key="pub.id" class="hover:bg-slate-50/80 transition-colors">
              <td class="px-6 py-3">
                <div class="flex items-center gap-3">
                  <div class="w-16 h-10 rounded-lg bg-slate-100 flex items-center justify-center overflow-hidden flex-shrink-0">
                    <svg class="w-5 h-5 text-slate-300" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M7 4v16M17 4v16M3 8h4m10 0h4M3 12h18M3 16h4m10 0h4M4 20h16a1 1 0 001-1V5a1 1 0 00-1-1H4a1 1 0 00-1 1v14a1 1 0 001 1z" />
                    </svg>
                  </div>
                  <div class="min-w-0">
                    <p class="text-sm font-medium text-slate-700 truncate">{{ pub.title }}</p>
                    <p class="text-xs text-slate-400 truncate">{{ pub.description }}</p>
                  </div>
                </div>
              </td>
              <td class="px-6 py-3">
                <div class="flex -space-x-2">
                  <div v-for="(pa, idx) in (pub.publishedAccounts || []).slice(0, 4)" :key="idx" class="w-7 h-7 rounded-full bg-white border-2 border-white flex items-center justify-center shadow-sm">
                    <img :src="getPlatformInfo(pa.platform).icon" :alt="pa.accountName" class="w-4 h-4 object-contain" />
                  </div>
                </div>
              </td>
              <td class="px-6 py-3">
                <span :class="['inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium', getStatusConfig(pub.status).bg, getStatusConfig(pub.status).textColor]">
                  <span :class="['w-1.5 h-1.5 rounded-full', getStatusConfig(pub.status).dot]"></span>
                  {{ getStatusConfig(pub.status).text }}
                </span>
              </td>
              <td class="px-6 py-3 text-sm text-slate-600">{{ formatNumber((pub.publishedAccounts || []).reduce((sum: number, pa: any) => sum + (pa.stats?.comments || 0), 0)) }}</td>
              <td class="px-6 py-3 text-sm text-slate-600">{{ formatNumber((pub.publishedAccounts || []).reduce((sum: number, pa: any) => sum + (pa.stats?.likes || 0), 0)) }}</td>
              <td class="px-6 py-3 text-sm text-slate-500">{{ pub.createdAt }}</td>
              <td class="px-6 py-3">
                <div class="flex items-center justify-end gap-2">
                  <button @click="viewDetail(pub.id)" class="px-3 py-1.5 text-xs font-medium text-slate-600 hover:text-indigo-600 hover:bg-indigo-50 rounded-lg transition-colors">详情</button>
                  <button @click="handleDelete(pub.id)" class="px-3 py-1.5 text-xs font-medium text-rose-600 hover:text-rose-700 hover:bg-rose-50 rounded-lg transition-colors">删除</button>
                </div>
              </td>
            </tr>
          </tbody>
        </table>
      </div>

      <!-- Empty State -->
      <div v-if="!loading && filteredPublications.length === 0" class="py-12 text-center">
        <div class="w-16 h-16 mx-auto mb-3 rounded-full bg-slate-100 flex items-center justify-center">
          <svg class="w-8 h-8 text-slate-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
          </svg>
        </div>
        <h3 class="text-sm font-medium text-slate-600 mb-1">暂无作品</h3>
        <p class="text-xs text-slate-400">点击上方按钮发布您的第一个作品</p>
      </div>
    </div>

    <!-- Publish Dialog -->
    <PublishDialog
      :show="showPublishDialog"
      :accounts="accounts"
      @close="showPublishDialog = false"
      @publish="handlePublish"
    />
  </div>
</template>
