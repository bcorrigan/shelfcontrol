import Vue from 'vue'
import VueRouter from 'vue-router'
import vuetify from '@/plugins/vuetify' // path to vuetify export
import App from './App.vue'
import axios from 'axios'
import epubjs from "epubjs";

Vue.config.productionTip = false
Vue.prototype.$axios = axios
Vue.prototype.$epubjs = epubjs
Vue.use(VueRouter)

import Books from './components/Books.vue' 
import Categories from './components/Categories.vue'

const router = new VueRouter({
  mode: 'history',
  base: __dirname,
  routes: [
    { path: '/books/:search', name: "books", component: Books },
    { path: '/books', name: "books", component: Books },
    { path: '/categories/:type', name: "categories", component: Categories, props: true  },
  ]
});

new Vue({
  router: router,
  vuetify,
  render: h => h(App),
}).$mount('#app')
