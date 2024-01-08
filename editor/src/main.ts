import "primevue/resources/themes/viva-light/theme.css"
import "primeicons/primeicons.css"
import "primeflex/primeflex.css"

import { createApp } from "vue"
import { createPinia } from "pinia"
import PrimeVue from "primevue/config"
import DataTable from "primevue/datatable"
import Column from "primevue/column"

import App from "./App.vue"
import router from "./router"

const app = createApp(App)

app.use(createPinia())
app.use(router)
app.use(PrimeVue)

app.component("PrimeDataTable", DataTable)
app.component("PrimeColumn", Column)

app.mount("#app")
