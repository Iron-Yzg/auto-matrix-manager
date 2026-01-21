import { createRouter, createWebHashHistory } from 'vue-router'
import AccountManagement from '../views/AccountManagement.vue'
import WorkPublication from '../views/WorkPublication.vue'
import PublicationDetail from '../views/PublicationDetail.vue'
import CommentDetail from '../views/CommentDetail.vue'
import Settings from '../views/Settings.vue'

const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    {
      path: '/',
      redirect: '/accounts'
    },
    {
      path: '/accounts',
      name: 'accounts',
      component: AccountManagement
    },
    {
      path: '/publications',
      name: 'publications',
      component: WorkPublication
    },
    {
      path: '/publications/:id',
      name: 'publication-detail',
      component: PublicationDetail,
      props: true
    },
    {
      path: '/comments',
      name: 'comment-detail',
      component: CommentDetail,
      props: route => ({ id: route.query.id, awemeId: route.query.awemeId, accountName: route.query.accountName })
    },
    {
      path: '/settings',
      name: 'settings',
      component: Settings
    }
  ]
})

export default router
