<script setup lang="ts">
import { ref } from 'vue'
import { PLATFORMS, type Account, type Platform } from '../types'

const accounts = ref<Account[]>([
  { id: '1', platform: 'douyin', accountName: '抖音测试号', avatar: 'https://api.dicebear.com/7.x/avataaars/svg?seed=douyin1', status: 'active', authorizedAt: '2024-01-15 10:30:00' },
  { id: '2', platform: 'xiaohongshu', accountName: '小红书运营号', avatar: 'https://api.dicebear.com/7.x/avataaars/svg?seed=xhs1', status: 'active', authorizedAt: '2024-01-14 15:20:00' },
  { id: '3', platform: 'kuaishou', accountName: '快手创作号', avatar: 'https://api.dicebear.com/7.x/avataaars/svg?seed=ks1', status: 'expired', authorizedAt: '2024-01-10 09:00:00' },
  { id: '4', platform: 'bilibili', accountName: 'B站UP主', avatar: 'https://api.dicebear.com/7.x/avataaars/svg?seed=bili1', status: 'active', authorizedAt: '2024-01-13 14:30:00' }
])

const getPlatformInfo = (platform: Platform) => PLATFORMS.find(p => p.id === platform)!

const getStatusConfig = (status: Account['status']) => {
  switch (status) {
    case 'active': return { text: '已授权', bg: 'bg-emerald-50', textColor: 'text-emerald-600', dot: 'bg-emerald-500' }
    case 'expired': return { text: '已过期', bg: 'bg-rose-50', textColor: 'text-rose-600', dot: 'bg-rose-500' }
    default: return { text: '待授权', bg: 'bg-amber-50', textColor: 'text-amber-600', dot: 'bg-amber-500' }
  }
}

const handleReauthorize = (account: Account) => {
  account.status = 'active'
  account.authorizedAt = new Date().toLocaleString('zh-CN')
}

const handleDelete = (id: string) => {
  const index = accounts.value.findIndex(a => a.id === id)
  if (index > -1) accounts.value.splice(index, 1)
}
</script>

<template>
  <div class="h-full flex flex-col p-6">
    <!-- Header -->
    <div class="flex items-center justify-between mb-4 flex-shrink-0">
      <button class="group flex items-center gap-2 px-4 py-2.5 bg-indigo-600 text-white rounded-xl hover:bg-indigo-700 transition-all duration-200">
        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
        </svg>
        <span class="font-medium">添加账号</span>
      </button>
    </div>

    <!-- Account Table -->
    <div class="flex-1 bg-white rounded-2xl border border-slate-200 overflow-hidden flex flex-col">
      <div class="overflow-y-auto flex-1">
        <table class="w-full">
          <thead class="bg-slate-50 border-b border-slate-200 sticky top-0">
            <tr>
              <th class="text-left px-6 py-3 text-xs font-semibold text-slate-500 uppercase tracking-wider">平台</th>
              <th class="text-left px-6 py-3 text-xs font-semibold text-slate-500 uppercase tracking-wider">账号名称</th>
              <th class="text-left px-6 py-3 text-xs font-semibold text-slate-500 uppercase tracking-wider">授权时间</th>
              <th class="text-left px-6 py-3 text-xs font-semibold text-slate-500 uppercase tracking-wider">状态</th>
              <th class="text-right px-6 py-3 text-xs font-semibold text-slate-500 uppercase tracking-wider">操作</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-slate-100">
            <tr v-for="account in accounts" :key="account.id" class="hover:bg-slate-50/80 transition-colors">
              <td class="px-6 py-3">
                <div class="flex items-center gap-3">
                  <div class="w-8 h-8 rounded-lg bg-slate-100 flex items-center justify-center overflow-hidden">
                    <img :src="getPlatformInfo(account.platform).icon" :alt="getPlatformInfo(account.platform).name" class="w-5 h-5 object-contain" />
                  </div>
                  <span class="text-sm font-medium text-slate-700">{{ getPlatformInfo(account.platform).name }}</span>
                </div>
              </td>
              <td class="px-6 py-3">
                <div class="flex items-center gap-3">
                  <img :src="account.avatar" :alt="account.accountName" class="w-7 h-7 rounded-full bg-slate-100" />
                  <span class="text-sm text-slate-700">{{ account.accountName }}</span>
                </div>
              </td>
              <td class="px-6 py-3 text-sm text-slate-500">{{ account.authorizedAt }}</td>
              <td class="px-6 py-3">
                <span :class="['inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium', getStatusConfig(account.status).bg, getStatusConfig(account.status).textColor]">
                  <span :class="['w-1.5 h-1.5 rounded-full', getStatusConfig(account.status).dot]"></span>
                  {{ getStatusConfig(account.status).text }}
                </span>
              </td>
              <td class="px-6 py-3">
                <div class="flex items-center justify-end gap-2">
                  <button @click="handleReauthorize(account)" class="px-3 py-1.5 text-xs font-medium text-slate-600 hover:text-slate-900 hover:bg-slate-100 rounded-lg transition-colors">重新授权</button>
                  <button @click="handleDelete(account.id)" class="px-3 py-1.5 text-xs font-medium text-rose-600 hover:text-rose-700 hover:bg-rose-50 rounded-lg transition-colors">删除</button>
                </div>
              </td>
            </tr>
          </tbody>
        </table>
      </div>

      <!-- Empty State -->
      <div v-if="accounts.length === 0" class="py-12 text-center">
        <div class="w-16 h-16 mx-auto mb-3 rounded-full bg-slate-100 flex items-center justify-center">
          <svg class="w-8 h-8 text-slate-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0zm6 3a2 2 0 11-4 0 2 2 0 014 0zM7 10a2 2 0 11-4 0 2 2 0 014 0z" />
          </svg>
        </div>
        <h3 class="text-sm font-medium text-slate-600 mb-1">暂无账号</h3>
        <p class="text-xs text-slate-400">将鼠标悬停在侧边栏"添加账号"上选择平台</p>
      </div>
    </div>
  </div>
</template>
