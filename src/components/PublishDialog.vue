<script setup lang="ts">
import { ref, computed, watch, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import Hls from 'hls.js'

interface Props {
  show: boolean
  accounts: Array<{
    id: string
    platform: string
    accountName: string
    avatar: string
    status: 'active' | 'expired' | 'pending'
  }>
}

const props = defineProps<Props>()

const emit = defineEmits<{
  close: []
  publish: [data: {
    title: string
    description: string
    videoPath: string
    coverPath: string | null
    accountIds: string[]
    platforms: string[]
    hashtags: string[][]
  }]
}>()

// Step state
const currentStep = ref(1)

// Video file state
const videoPath = ref('')
const videoBase64 = ref('')
const videoUrl = computed(() => videoBase64.value ? `data:video/mp4;base64,${videoBase64.value}` : '')

// Cover file state
const coverPath = ref('')
const coverBase64 = ref('')
const coverUrl = computed(() => coverBase64.value ? `data:image;base64,${coverBase64.value}` : '')

// Video player refs
const videoElement = ref<HTMLVideoElement | null>(null)
let hls: Hls | null = null

// Form state
const title = ref('')
const description = ref('')
const topics = ref<string[]>([])
const newTopic = ref('')
const selectedAccounts = ref<string[]>([])

// Initialize video player with HLS support
const initVideoPlayer = () => {
  const video = videoElement.value
  if (!video || !videoBase64.value) return

  if (hls) {
    hls.destroy()
    hls = null
  }

  const videoSrc = videoUrl.value

  if (videoSrc.endsWith('.m3u8')) {
    if (Hls.isSupported()) {
      hls = new Hls()
      hls.loadSource(videoSrc)
      hls.attachMedia(video)
      hls.on(Hls.Events.MANIFEST_PARSED, () => {
        video.play().catch(() => {})
      })
      hls.on(Hls.Events.ERROR, (_, data) => {
        if (data.fatal) {
          console.error('HLS fatal error:', data)
        }
      })
    } else if (video.canPlayType('application/vnd.apple.mpegurl')) {
      video.src = videoSrc
      video.addEventListener('loadedmetadata', () => {
        video.play().catch(() => {})
      })
    }
  } else {
    video.src = videoSrc
  }
}

// Clean up HLS on unmount
const cleanupPlayer = () => {
  if (hls) {
    hls.destroy()
    hls = null
  }
}

// Trigger video file selection
const triggerVideoInput = async () => {
  try {
    const result = await invoke<{ path: string; name: string; content: string; mime_type: string } | null>('select_file_with_content', {
      title: '选择视频文件',
      fileType: 'video',
      filters: ['mp4,mov,avi,mkv,webm'],
    })

    if (result && result.path) {
      videoPath.value = result.path
      videoBase64.value = result.content
    }
  } catch (e) {
    console.error('Failed to select video:', e)
    alert('选择视频失败')
  }
}

// Trigger cover file selection
const triggerCoverInput = async () => {
  try {
    const result = await invoke<{ path: string; name: string; content: string; mime_type: string } | null>('select_file_with_content', {
      title: '选择封面图片',
      fileType: 'image',
      filters: ['png,jpg,jpeg,webp'],
    })

    if (result && result.path) {
      coverPath.value = result.path
      coverBase64.value = result.content
    }
  } catch (e) {
    console.error('Failed to select cover:', e)
    alert('选择封面失败')
  }
}

// Clear cover selection
const clearCover = () => {
  coverPath.value = ''
  coverBase64.value = ''
}

// Watch for video changes
watch(videoBase64, (newVal) => {
  if (newVal) {
    setTimeout(initVideoPlayer, 100)
  } else {
    cleanupPlayer()
  }
})

// Account selection
const toggleAccount = (accountId: string) => {
  console.log('[Debug] toggleAccount called:', { accountId, selectedCount: selectedAccounts.value.length })
  const index = selectedAccounts.value.indexOf(accountId)
  if (index > -1) {
    selectedAccounts.value.splice(index, 1)
  } else {
    selectedAccounts.value.push(accountId)
  }
  console.log('[Debug] After toggle:', { selectedCount: selectedAccounts.value.length, selectedIds: selectedAccounts.value })
}

// Check if all active accounts are selected
const isAllSelected = computed(() => {
  const activeIds = props.accounts
    .filter(a => a.status === 'active')
    .map(a => a.id)
  return activeIds.length > 0 && activeIds.every(id => selectedAccounts.value.includes(id))
})

// Check if some accounts are selected (partial selection)
const hasPartialSelection = computed(() => {
  return selectedAccounts.value.length > 0 && !isAllSelected.value
})

// Toggle select all
const toggleSelectAll = () => {
  if (isAllSelected.value) {
    selectedAccounts.value = []
  } else {
    selectedAccounts.value = props.accounts
      .filter(a => a.status === 'active')
      .map(a => a.id)
  }
}

// Topic management
const addTopic = () => {
  if (newTopic.value.trim() && !topics.value.includes(newTopic.value.trim())) {
    topics.value.push(newTopic.value.trim())
    newTopic.value = ''
  }
}

const removeTopic = (index: number) => {
  topics.value.splice(index, 1)
}

// Navigation
const nextStep = () => {
  if (currentStep.value < 2 && videoPath.value) {
    currentStep.value++
  }
}

const prevStep = () => {
  if (currentStep.value > 1) {
    currentStep.value--
  }
}

// Publish
const handlePublish = () => {
  if (!videoPath.value || selectedAccounts.value.length === 0) return

  const platforms = selectedAccounts.value.map(accountId => {
    const account = props.accounts.find(a => a.id === accountId)
    return account?.platform || 'douyin'
  })

  const hashtags = selectedAccounts.value.map(() => topics.value)

  emit('publish', {
    title: title.value || '未命名视频',
    description: description.value,
    videoPath: videoPath.value,
    coverPath: coverPath.value || null,
    accountIds: selectedAccounts.value,
    platforms,
    hashtags,
  })
}

// Close and reset
const close = () => {
  reset()
  emit('close')
}

const reset = () => {
  currentStep.value = 1
  cleanupPlayer()
  videoPath.value = ''
  videoBase64.value = ''
  coverPath.value = ''
  coverBase64.value = ''
  title.value = ''
  description.value = ''
  topics.value = []
  newTopic.value = ''
  selectedAccounts.value = []
}

// Watch for show changes to reset
watch(() => props.show, (newVal) => {
  if (!newVal) {
    reset()
  }
})

// Cleanup on unmount
onUnmounted(() => {
  cleanupPlayer()
})
</script>

<template>
  <Teleport to="body">
    <div v-if="show" class="fixed inset-0 bg-slate-900/60 backdrop-blur-sm flex items-center justify-center z-50 p-4" @click.self="close">
      <div class="bg-white rounded-2xl w-full max-w-4xl shadow-2xl max-h-[90vh] overflow-hidden flex flex-col">
        <!-- Header -->
        <div class="px-8 py-6 border-b border-slate-100 flex items-center justify-between flex-shrink-0">
          <div class="flex items-center gap-4">
            <div class="flex items-center gap-2">
              <span :class="['w-8 h-8 rounded-full flex items-center justify-center text-sm font-medium', currentStep >= 1 ? 'bg-indigo-600 text-white' : 'bg-slate-100 text-slate-400']">1</span>
              <span :class="['text-sm', currentStep >= 1 ? 'text-slate-800 font-medium' : 'text-slate-400']">填写信息</span>
            </div>
            <div class="w-12 h-px bg-slate-200"></div>
            <div class="flex items-center gap-2">
              <span :class="['w-8 h-8 rounded-full flex items-center justify-center text-sm font-medium', currentStep >= 2 ? 'bg-indigo-600 text-white' : 'bg-slate-100 text-slate-400']">2</span>
              <span :class="['text-sm', currentStep >= 2 ? 'text-slate-800 font-medium' : 'text-slate-400']">选择账号</span>
            </div>
          </div>
          <button @click="close" class="w-8 h-8 flex items-center justify-center rounded-lg hover:bg-slate-100 transition-colors">
            <svg class="w-5 h-5 text-slate-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>

        <!-- Step 1: Cover & Video -->
        <div v-if="currentStep === 1" class="p-6 overflow-y-auto flex-1">
          <!-- Cover & Video Row -->
          <div class="flex gap-4 mb-4">
            <!-- Cover Section -->
            <div class="w-1/2">
              <label class="block text-sm font-semibold text-slate-700 mb-2">视频封面</label>
              <div @click="triggerCoverInput" class="aspect-video rounded-xl bg-slate-100 border-2 border-dashed border-slate-300 flex items-center justify-center overflow-hidden cursor-pointer hover:border-indigo-400 transition-colors relative">
                <img v-if="coverUrl" :src="coverUrl" class="w-full h-full object-contain bg-slate-50" alt="Cover" />
                <div v-else class="text-center">
                  <svg class="w-8 h-8 text-slate-400 mx-auto mb-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z" />
                  </svg>
                  <p class="text-xs text-slate-400">点击选择封面</p>
                </div>

                <!-- Clear button when cover is selected (top right corner) -->
                <button v-if="coverPath" @click.stop="clearCover" class="absolute top-2 right-2 w-6 h-6 bg-black/50 hover:bg-black/70 rounded-full flex items-center justify-center z-20 transition-colors">
                  <svg class="w-4 h-4 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                  </svg>
                </button>
              </div>
              <!-- Full path display -->
              <p v-if="coverPath" class="text-xs text-slate-500 mt-1 truncate">{{ coverPath }}</p>
            </div>

            <!-- Video Section -->
            <div class="w-1/2">
              <label class="block text-sm font-semibold text-slate-700 mb-2">选择视频</label>
              <div class="aspect-video rounded-xl bg-slate-900 border border-slate-700 flex items-center justify-center overflow-hidden relative group">
                <!-- Video Player (点击不触发选择) -->
                <video v-if="videoUrl" ref="videoElement" @click.stop class="w-full h-full" controls playsinline></video>

                <!-- Empty State (点击触发选择) -->
                <div v-if="!videoPath" @click="triggerVideoInput" class="text-center cursor-pointer">
                  <svg class="w-10 h-10 text-slate-500 mx-auto mb-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M7 4v16M17 4v16M3 8h4m10 0h4M3 12h18M3 16h4m10 0h4M4 20h16a1 1 0 001-1V5a1 1 0 00-1-1H4a1 1 0 00-1 1v14a1 1 0 001 1z" />
                  </svg>
                  <p class="text-xs text-slate-400 mb-2">点击选择视频</p>
                </div>

                <!-- Reselect button (small, bottom right) -->
                <button v-if="videoPath" @click.stop="triggerVideoInput" class="absolute bottom-2 right-2 px-2 py-1 bg-black/50 hover:bg-black/70 rounded text-white text-xs flex items-center gap-1 z-20 transition-colors">
                  <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                  </svg>
                  重新选择
                </button>
              </div>

              <!-- Full path display -->
              <p v-if="videoPath" class="text-xs text-slate-500 mt-1 truncate">{{ videoPath }}</p>
            </div>
          </div>

          <!-- Title -->
          <div class="mb-4">
            <label class="block text-sm font-semibold text-slate-700 mb-2">作品标题</label>
            <input v-model="title" type="text" placeholder="输入作品标题" class="w-full px-4 py-2.5 bg-slate-50 border border-slate-200 rounded-xl focus:outline-none focus:ring-2 focus:ring-indigo-500/10 focus:bg-white" />
          </div>

          <!-- Description -->
          <div class="mb-4">
            <label class="block text-sm font-semibold text-slate-700 mb-2">作品描述</label>
            <textarea v-model="description" placeholder="输入作品描述，支持换行" rows="3" class="w-full px-4 py-2.5 bg-slate-50 border border-slate-200 rounded-xl focus:outline-none focus:ring-2 focus:ring-indigo-500/10 focus:bg-white resize-none"></textarea>
          </div>

          <!-- Topics -->
          <div class="mb-4">
            <label class="block text-sm font-semibold text-slate-700 mb-2">话题标签</label>
            <div class="flex gap-2 mb-2 flex-wrap">
              <span v-for="(topic, index) in topics" :key="index" class="inline-flex items-center gap-1 px-3 py-1 bg-indigo-50 text-indigo-600 rounded-full text-sm">
                #{{ topic }}
                <button @click="removeTopic(index)" class="hover:text-indigo-800">
                  <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                  </svg>
                </button>
              </span>
            </div>
            <div class="flex gap-2">
              <input v-model="newTopic" type="text" placeholder="添加话题，按回车确认" class="flex-1 px-4 py-2 bg-slate-50 border border-slate-200 rounded-xl focus:outline-none focus:ring-2 focus:ring-indigo-500/10 focus:bg-white" @keyup.enter="addTopic" />
              <button @click="addTopic" class="px-4 py-2 bg-slate-100 text-slate-600 rounded-xl hover:bg-slate-200 transition-colors">添加</button>
            </div>
          </div>
        </div>

        <!-- Step 2: Account Selection -->
        <div v-if="currentStep === 2" class="overflow-y-auto flex-1">
          <!-- Table Header with Select All Checkbox -->
          <div class="px-6 py-3 bg-slate-50 border-b border-slate-200 sticky top-0">
            <div class="flex items-center gap-3">
              <label class="relative flex items-center cursor-pointer">
                <input
                  type="checkbox"
                  :checked="selectedAccounts.length > 0"
                  :indeterminate="hasPartialSelection"
                  @change="toggleSelectAll"
                  class="peer w-4 h-4 border-2 border-slate-300 rounded cursor-pointer appearance-none checked:border-indigo-600 checked:bg-indigo-600 hover:border-indigo-400 transition-colors"
                />
                <!-- 自定义 indeterminate 样式 -->
                <svg v-if="hasPartialSelection" class="absolute left-0 w-4 h-4 pointer-events-none text-indigo-600" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3">
                  <path d="M5 12h14" stroke-linecap="round" />
                </svg>
                <!-- 自定义 checked 样式 -->
                <svg v-else-if="selectedAccounts.length > 0" class="absolute left-0 w-4 h-4 pointer-events-none text-white" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3">
                  <path d="M4 12l6 6L20 6" stroke-linecap="round" />
                </svg>
              </label>
              <span class="text-xs font-semibold text-slate-500 uppercase tracking-wider">选择发布账号</span>
              <span class="text-xs text-slate-400">({{ selectedAccounts.length }} 个已选)</span>
            </div>
          </div>

          <!-- Account List -->
          <div class="divide-y divide-slate-100">
            <div
              v-for="account in accounts"
              :key="account.id"
              class="flex items-center gap-4 px-6 py-3 transition-colors"
              :class="[
                account.status === 'active' ? 'hover:bg-slate-50 cursor-pointer' : 'opacity-50 cursor-not-allowed',
                selectedAccounts.includes(account.id) ? 'bg-indigo-50/50' : ''
              ]"
              @click="account.status === 'active' && toggleAccount(account.id)"
            >
              <input
                type="checkbox"
                :checked="selectedAccounts.includes(account.id)"
                :disabled="account.status !== 'active'"
                class="w-4 h-4 text-indigo-600 rounded border-slate-300 focus:ring-indigo-500 cursor-pointer"
                @change="toggleAccount(account.id)"
                @click.stop
              />
              <div class="w-10 h-10 rounded-full bg-white border border-slate-200 flex items-center justify-center overflow-hidden flex-shrink-0">
                <img v-if="account.avatar" :src="account.avatar" :alt="account.accountName" class="w-full h-full object-cover" />
                <span v-else class="text-sm text-slate-400">{{ account.accountName.charAt(0) }}</span>
              </div>
              <div class="flex-1 min-w-0">
                <p class="text-sm font-medium text-slate-700 truncate">{{ account.accountName }}</p>
                <p class="text-xs text-slate-400 capitalize">{{ account.platform }}</p>
              </div>
              <span
                :class="['inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium', account.status === 'active' ? 'bg-emerald-50 text-emerald-600' : account.status === 'expired' ? 'bg-rose-50 text-rose-600' : 'bg-amber-50 text-amber-600']"
              >
                <span :class="['w-1.5 h-1.5 rounded-full', account.status === 'active' ? 'bg-emerald-500' : account.status === 'expired' ? 'bg-rose-500' : 'bg-amber-500']"></span>
                {{ account.status === 'active' ? '已授权' : account.status === 'expired' ? '已过期' : '待授权' }}
              </span>
            </div>

            <!-- Empty State -->
            <div v-if="accounts.length === 0" class="py-12 text-center">
              <div class="w-16 h-16 mx-auto mb-3 rounded-full bg-slate-100 flex items-center justify-center">
                <svg class="w-8 h-8 text-slate-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0zm6 3a2 2 0 11-4 0 2 2 0 014 0zM7 10a2 2 0 11-4 0 2 2 0 014 0z" />
                </svg>
              </div>
              <h3 class="text-sm font-medium text-slate-600 mb-1">暂无账号</h3>
              <p class="text-xs text-slate-400">请先在账号管理中添加账号</p>
            </div>
          </div>
        </div>

        <!-- Footer -->
        <div class="px-8 py-6 border-t border-slate-100 flex items-center gap-3 flex-shrink-0">
          <button v-if="currentStep > 1" @click="prevStep" class="px-6 py-3 text-sm font-medium text-slate-600 hover:text-slate-900 hover:bg-slate-100 rounded-xl transition-colors">上一步</button>
          <button v-if="currentStep === 1" @click="close" class="px-6 py-3 text-sm font-medium text-slate-600 hover:text-slate-900 hover:bg-slate-100 rounded-xl transition-colors">取消</button>
          <button v-if="currentStep === 1" @click="nextStep" :disabled="!videoPath" class="flex-1 px-6 py-3 text-sm font-medium text-white bg-indigo-600 rounded-xl hover:bg-indigo-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed">下一步</button>
          <button v-if="currentStep === 2" @click="handlePublish" :disabled="selectedAccounts.length === 0" class="flex-1 px-6 py-3 text-sm font-medium text-white bg-indigo-600 rounded-xl hover:bg-indigo-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed">发布</button>
        </div>
      </div>
    </div>
  </Teleport>
</template>
