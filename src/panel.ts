import { createApp } from 'vue'
import { createPinia } from 'pinia'

import Panel from './Panel.vue'
import './styles.css'

createApp(Panel).use(createPinia()).mount('#panel')
