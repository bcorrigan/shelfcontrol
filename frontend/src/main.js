import { createApp } from 'vue';
import { createWebHistory, createRouter } from 'vue-router';
import vuetify from '@/plugins/vuetify'; // path to vuetify export
import App from './App.vue';
import axios from 'axios';
import epubjs from "@parkdoeui/epubjs";

const app = createApp(App);

app.config.globalProperties.$axios = axios;
app.config.globalProperties.$epubjs = epubjs;
//app.use(VueRouter)

import Books from './components/Books.vue';
import Categories from './components/Categories.vue';

const router = createRouter({
  history: createWebHistory(import.meta.env.VITE_BASE_URL),
  routes: [
    { path: '/books/:search?', name: "books", component: Books, props: true },
    { path: '/categories/:type?', name: "categories", component: Categories, props: true  },
  ]
});

app.use(router);
app.use(vuetify);

router.isReady().then(() => app.mount('#app'));

/*new Vue({
  router: router,
  vuetify,
  render: h => h(App),
}).$mount('#app')*/
