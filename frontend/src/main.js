import Vue from 'vue'
import vuetify from '@/plugins/vuetify' // path to vuetify export
import App from './App.vue'
import axios from 'axios'
import epubjs from "epubjs";

Vue.config.productionTip = false
Vue.prototype.$axios = axios
Vue.prototype.$epubjs = epubjs

new Vue({
  vuetify,
  render: h => h(App),
}).$mount('#app')
