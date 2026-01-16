<script setup lang="ts">
import { ref } from 'vue'
import { PLATFORMS, type Platform } from '../types'

interface PlatformCredentials {
  accessKey: string
  accessSecret: string
}

const credentials = ref<Record<Platform, PlatformCredentials>>({
  douyin: { accessKey: '', accessSecret: '' },
  xiaohongshu: { accessKey: '', accessSecret: '' },
  kuaishou: { accessKey: '', accessSecret: '' },
  bilibili: { accessKey: '', accessSecret: '' }
})

const handleSave = (platform: Platform) => {
  console.log('Saving credentials for:', platform, credentials.value[platform])
}

const handleReset = (platform: Platform) => {
  credentials.value[platform].accessKey = ''
  credentials.value[platform].accessSecret = ''
}
</script>

<template>
  <div class="h-full flex flex-col p-6">
    <div class="mb-4 flex-shrink-0">
      <h1 class="text-lg font-semibold text-slate-800">平台设置</h1>
      <p class="text-sm text-slate-500 mt-1">配置各平台的 API 凭证信息</p>
    </div>

    <div class="flex-1 bg-white rounded-2xl border border-slate-200 overflow-hidden flex flex-col">
      <div class="overflow-y-auto flex-1 p-6">
        <div class="grid gap-6">
          <div v-for="platform in PLATFORMS" :key="platform.id" class="border border-slate-200 rounded-xl p-5">
            <div class="flex items-center gap-3 mb-4">
              <div class="w-10 h-10 rounded-lg flex items-center justify-center" :style="{ backgroundColor: platform.color + '15' }">
                <img :src="platform.icon" :alt="platform.name" class="w-6 h-6 object-contain" />
              </div>
              <div>
                <h3 class="text-sm font-semibold text-slate-800">{{ platform.name }}</h3>
                <p class="text-xs text-slate-400">配置 {{ platform.name }} 开放平台 API 凭证</p>
              </div>
            </div>

            <div class="space-y-3">
              <div>
                <label class="block text-xs font-medium text-slate-600 mb-1.5">Access Key</label>
                <input
                  v-model="credentials[platform.id].accessKey"
                  type="text"
                  :placeholder="`输入 ${platform.name} Access Key`"
                  class="w-full px-3 py-2 bg-slate-50 border border-slate-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-indigo-500/10 focus:bg-white"
                />
              </div>
              <div>
                <label class="block text-xs font-medium text-slate-600 mb-1.5">Access Secret</label>
                <input
                  v-model="credentials[platform.id].accessSecret"
                  type="password"
                  :placeholder="`输入 ${platform.name} Access Secret`"
                  class="w-full px-3 py-2 bg-slate-50 border border-slate-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-indigo-500/10 focus:bg-white"
                />
              </div>
            </div>

            <div class="flex gap-2 mt-4 pt-4 border-t border-slate-100">
              <button
                @click="handleSave(platform.id)"
                class="flex-1 px-3 py-2 bg-indigo-600 text-white text-sm font-medium rounded-lg hover:bg-indigo-700 transition-colors"
              >
                保存
              </button>
              <button
                @click="handleReset(platform.id)"
                class="px-3 py-2 bg-slate-100 text-slate-600 text-sm font-medium rounded-lg hover:bg-slate-200 transition-colors"
              >
                重置
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
