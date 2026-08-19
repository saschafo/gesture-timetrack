import { createApp } from 'vue'

import Overlay from './Overlay.vue'
import './styles.css'

// Das Overlay kommt ohne Store aus: es hält keinen Zustand, sondern meldet
// erkannte Gesten sofort an das Backend.
createApp(Overlay).mount('#overlay')
