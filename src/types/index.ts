// Platform types
export type Platform = 'douyin' | 'xiaohongshu' | 'kuaishou' | 'bilibili'

export interface PlatformInfo {
  id: Platform
  name: string
  icon: string
  color: string
}

export const PLATFORMS: PlatformInfo[] = [
  { id: 'douyin', name: '抖音', icon: '/src/assets/icons/douyin.png', color: '#000000' },
  { id: 'xiaohongshu', name: '小红书', icon: '/src/assets/icons/xiaohongshu.ico', color: '#FE2C55' },
  { id: 'kuaishou', name: '快手', icon: '/src/assets/icons/kuaishou.ico', color: '#FF4906' },
  { id: 'bilibili', name: 'B站', icon: '/src/assets/icons/bilibili.ico', color: '#00A1D6' },
]

// Account types
export interface Account {
  id: string
  platform: Platform
  accountName: string
  avatar: string
  status: 'active' | 'expired' | 'pending'
  authorizedAt: string
}

// Publication types
export interface Publication {
  id: string
  videoPath: string
  coverPath: string
  title: string
  description: string
  status: 'draft' | 'publishing' | 'completed' | 'failed'
  createdAt: string
  publishedAccounts: PublishedAccount[]
}

export interface PublishedAccount {
  id: string
  accountId: string
  platform: Platform
  accountName: string
  status: 'pending' | 'publishing' | 'success' | 'failed'
  publishUrl?: string
  publishTime?: string
  stats: {
    comments: number
    likes: number
    favorites: number
    shares: number
  }
}

// Comment types
export interface Comment {
  id: string
  username: string
  avatar: string
  content: string
  time: string
  likes: number
  replyCount: number
}
