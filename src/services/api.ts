// Tauri API Service
// 使用 Tauri invoke 调用后端命令

import { invoke } from '@tauri-apps/api/core'

// Types matching the backend UserAccount
export interface UserAccount {
  id: string
  username: string
  nickname: string
  avatar_url: string
  platform: PlatformType
  params: string
  status: AccountStatus
  created_at: string
}

export type PlatformType = 'Douyin' | 'Xiaohongshu' | 'Kuaishou' | 'Bilibili'
export type AccountStatus = 'Active' | 'Expired' | 'Pending'

export interface PlatformInfo {
  id: string
  name: string
  icon: string
  color: string
}

// Publication Task types (main + sub table structure)
export interface PublicationTaskWithAccounts {
  id: string
  title: string
  description: string
  videoPath: string
  coverPath: string
  hashtags: string[]
  status: PublicationStatus
  createdAt: string
  publishedAt: string
  accounts: PublicationAccountDetail[]
}

export interface PublicationAccountDetail {
  id: string
  publicationTaskId: string
  accountId: string
  accountName: string  // 冗余的账号名称
  platform: PlatformType
  // Note: title/description/hashtags are only in the main table now
  status: PublicationStatus
  createdAt: string
  publishedAt: string | null
  publishUrl: string | null
  stats: {
    comments: number
    likes: number
    favorites: number
    shares: number
  }
  message: string | null  // 发布失败原因
  itemId: string | null   // 发布成功的视频ID
}

export interface PublicationTaskDetail {
  id: string
  title: string
  description: string
  videoPath: string
  coverPath: string
  hashtags: string[]
  status: PublicationStatus
  createdAt: string
  publishedAt: string
  accounts: PublicationAccountDetail[]
}

export type PublicationStatus = 'Draft' | 'Publishing' | 'Completed' | 'Failed'

// Platform list (can be fetched from backend in the future)
export const PLATFORMS: PlatformInfo[] = [
  { id: 'douyin', name: '抖音', icon: '/src/assets/icons/douyin.png', color: '#000000' },
  { id: 'xiaohongshu', name: '小红书', icon: '/src/assets/icons/xiaohongshu.ico', color: '#FE2C55' },
  { id: 'kuaishou', name: '快手', icon: '/src/assets/icons/kuaohou.ico', color: '#FF4906' },
  { id: 'bilibili', name: 'B站', icon: '/src/assets/icons/bilibili.ico', color: '#00A1D6' },
]

// API Functions

/**
 * Get supported platforms
 */
export async function getSupportedPlatforms(): Promise<PlatformInfo[]> {
  try {
    return await invoke<PlatformInfo[]>('get_supported_platforms')
  } catch (error) {
    console.error('Failed to get platforms:', error)
    return PLATFORMS
  }
}

/**
 * Get accounts for a platform
 */
export async function getAccounts(platform: string): Promise<UserAccount[]> {
  try {
    return await invoke<UserAccount[]>('get_accounts', { platform })
  } catch (error) {
    console.error('Failed to get accounts:', error)
    throw error
  }
}

/**
 * Get all accounts across all platforms
 */
export async function getAllAccounts(): Promise<UserAccount[]> {
  try {
    return await invoke<UserAccount[]>('get_all_accounts')
  } catch (error) {
    console.error('Failed to get all accounts:', error)
    throw error
  }
}

/**
 * Add a new account
 */
export async function addAccount(
  platform: string,
  username: string,
  nickname: string,
  avatarUrl: string,
  params: string
): Promise<UserAccount> {
  try {
    return await invoke<UserAccount>('add_account', {
      platform,
      username,
      nickname,
      avatar_url: avatarUrl,
      params,
    })
  } catch (error) {
    console.error('Failed to add account:', error)
    throw error
  }
}

/**
 * Delete an account
 */
export async function deleteAccount(accountId: string): Promise<boolean> {
  try {
    return await invoke<boolean>('delete_account', { accountId })
  } catch (error) {
    console.error('Failed to delete account:', error)
    throw error
  }
}

/**
 * Get publications for an account
 */
export async function getPublications(
  platform: string,
  accountId: string
): Promise<any[]> {
  try {
    return await invoke<any[]>('get_publications', { platform, accountId })
  } catch (error) {
    console.error('Failed to get publications:', error)
    throw error
  }
}

/**
 * Get all publication tasks with their account details
 */
export async function getPublicationTasks(): Promise<PublicationTaskWithAccounts[]> {
  try {
    return await invoke<PublicationTaskWithAccounts[]>('get_publication_tasks')
  } catch (error) {
    console.error('Failed to get publication tasks:', error)
    throw error
  }
}

/**
 * Get a single publication task with its account details
 */
export async function getPublicationTask(taskId: string): Promise<PublicationTaskWithAccounts | null> {
  try {
    return await invoke<PublicationTaskWithAccounts | null>('get_publication_task', { taskId })
  } catch (error) {
    console.error('Failed to get publication task:', error)
    throw error
  }
}

/**
 * Create a publication task with account details (main + sub tables)
 */
export async function createPublicationTask(
  title: string,
  description: string,
  videoPath: string,
  coverPath: string | null,
  accountIds: string[],
  platforms: string[],
  hashtags: string[][]
): Promise<PublicationTaskWithAccounts> {
  try {
    return await invoke<PublicationTaskWithAccounts>('create_publication_task', {
      title,
      description,
      videoPath,
      coverPath,
      accountIds,
      platforms,
      hashtags,
    })
  } catch (error) {
    console.error('Failed to create publication task:', error)
    throw error
  }
}

/**
 * Delete a publication task and all its account details
 */
export async function deletePublicationTask(taskId: string): Promise<boolean> {
  try {
    return await invoke<boolean>('delete_publication_task', { taskId })
  } catch (error) {
    console.error('Failed to delete publication task:', error)
    throw error
  }
}

/**
 * Get a publication task with all account details by ID
 */
export async function getPublicationTaskWithAccounts(taskId: string): Promise<PublicationTaskDetail | null> {
  try {
    return await invoke<PublicationTaskDetail | null>('get_publication_task_with_accounts', { taskId })
  } catch (error) {
    console.error('Failed to get publication task with accounts:', error)
    throw error
  }
}

/**
 * Get a single publication account detail by ID
 * 根据ID获取单个作品账号发布详情
 */
export async function getPublicationAccountDetail(detailId: string): Promise<PublicationAccountDetail | null> {
  try {
    return await invoke<PublicationAccountDetail | null>('get_publication_account_detail', { detailId })
  } catch (error) {
    console.error('Failed to get publication account detail:', error)
    throw error
  }
}

/**
 * Result of publishing a task
 */
export interface PublishTaskResult {
  success: boolean
  detailId: string
  publishUrl: string | null
  error: string | null
}

/**
 * Result of publishing with progress info
 */
export interface PublishProgressResult {
  totalAccounts: number
  completedAccounts: number
  successCount: number
  failedCount: number
  results: PublishTaskResult[]
}

/**
 * Publish a publication task to all accounts (concurrent/async)
 */
export async function publishPublicationTask(
  taskId: string,
  title: string,
  description: string,
  videoPath: string,
  hashtags: string[]
): Promise<PublishProgressResult> {
  try {
    return await invoke<PublishProgressResult>('publish_publication_task', {
      taskId,
      title,
      description,
      videoPath,
      hashtags,
    })
  } catch (error) {
    console.error('Failed to publish task:', error)
    throw error
  }
}

/**
 * Retry publishing for failed or pending accounts
 */
export async function retryPublicationTask(
  taskId: string
): Promise<PublishProgressResult> {
  try {
    return await invoke<PublishProgressResult>('retry_publication_task', { taskId })
  } catch (error) {
    console.error('Failed to retry task:', error)
    throw error
  }
}

/**
 * Legacy: Get all publications across all accounts (uses old structure)
 */
export async function getAllPublications(): Promise<any[]> {
  try {
    return await invoke<any[]>('get_publication_tasks')
  } catch (error) {
    console.error('Failed to get all publications:', error)
    throw error
  }
}

/**
 * Legacy: Save a publication draft (uses old structure)
 */
export async function savePublication(
  title: string,
  description: string,
  videoPath: string,
  coverPath: string | null,
  accountIds: string[],
  platforms: string[],
  hashtags: string[][]
): Promise<PublicationTaskWithAccounts> {
  try {
    return await invoke<PublicationTaskWithAccounts>('create_publication_task', {
      title,
      description,
      videoPath,
      coverPath,
      accountIds,
      platforms,
      hashtags,
    })
  } catch (error) {
    console.error('Failed to save publication:', error)
    throw error
  }
}

/**
 * Publish a video
 */
export async function publishVideo(
  platform: string,
  accountId: string,
  videoPath: string,
  title: string,
  description: string,
  hashtags: string[]
): Promise<any> {
  try {
    return await invoke('publish_video', {
      platform,
      accountId,
      videoPath,
      title,
      description,
      hashtags,
    })
  } catch (error) {
    console.error('Failed to publish video:', error)
    throw error
  }
}

/**
 * Publish a saved publication
 */
export async function publishSavedVideo(
  publicationId: string
): Promise<any> {
  try {
    return await invoke('publish_saved_video', { publicationId })
  } catch (error) {
    console.error('Failed to publish saved video:', error)
    throw error
  }
}

// Helper functions for type conversion

/**
 * Convert backend UserAccount to frontend Account format
 */
export function toFrontendAccount(account: UserAccount): {
  id: string
  platform: string
  accountName: string
  avatar: string
  status: 'active' | 'expired' | 'pending'
  authorizedAt: string
} {
  return {
    id: account.id,
    platform: account.platform.toLowerCase(),
    accountName: account.nickname || account.username,
    avatar: account.avatar_url,
    status: account.status.toLowerCase() as 'active' | 'expired' | 'pending',
    authorizedAt: account.created_at,
  }
}

/**
 * Convert platform type to lowercase string
 */
export function toPlatformString(platform: PlatformType): string {
  return platform.toLowerCase()
}

// ============================================================================
// Browser Automation Functions
// 浏览器自动化功能
// ============================================================================

/**
 * Browser authentication status
 */
export interface BrowserAuthStatus {
  step: string
  message: string
  currentUrl: string
  screenshot: string | null
  needPoll: boolean
  cookie: string
  localStorage: string
  nickname: string
  avatarUrl: string
  thirdId: string
  secUid: string
  error: string | null
}

/**
 * Start browser authentication flow for a platform
 * This launches a headless browser and navigates to the platform's login page
 * If accountId is provided, it will update the existing account instead of creating a new one
 */
export async function startBrowserAuth(platform: string, accountId?: string): Promise<BrowserAuthStatus> {
  console.log('[API] Starting browser auth for platform:', platform, 'accountId:', accountId)
  try {
    const result = await invoke<BrowserAuthStatus>('start_browser_auth', {
      platform,
      accountId: accountId || null
    })
    console.log('[API] Browser auth started successfully:', result)
    return result
  } catch (error: any) {
    console.error('[API] Failed to start browser auth:', error)
    // 返回一个错误状态，让前端显示错误信息
    throw error
  }
}

/**
 * Check browser authentication status and extract credentials if logged in
 * This should be called periodically to poll for login completion
 */
export async function checkBrowserAuthStatus(): Promise<BrowserAuthStatus> {
  try {
    return await invoke<BrowserAuthStatus>('check_browser_auth_status')
  } catch (error) {
    console.error('Failed to check auth status:', error)
    throw error
  }
}

/**
 * Cancel browser authentication
 */
export async function cancelBrowserAuth(): Promise<void> {
  try {
    await invoke('cancel_browser_auth')
  } catch (error) {
    console.error('Failed to cancel auth:', error)
    throw error
  }
}
