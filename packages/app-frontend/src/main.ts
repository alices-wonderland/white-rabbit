import "reflect-metadata";
import { createApp, markRaw } from "vue";
import App from "./App.vue";
import {
  agGridPlugin,
  router,
  vuetifyPlugin,
  i18nPlugin,
  KEY_API_SERVICE,
} from "@white-rabbit/components";
import { apiService, authService } from "./services";
import { createPinia } from "pinia";
import "./index.css";

const app = createApp(App);
app.provide(KEY_API_SERVICE, apiService);

const pinia = createPinia();
pinia.use(() => {
  // We cannot use inject in router.forEach, since it's not in Vue scope
  return { authService: markRaw(authService), router: markRaw(router) };
});

app.use(pinia);
app.use(agGridPlugin);
app.use(vuetifyPlugin);
app.use(i18nPlugin);
app.use(router);
app.mount("#app");
