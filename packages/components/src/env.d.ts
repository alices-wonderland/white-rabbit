/// <reference types="vite/client" />
import "pinia";
import { AuthService } from "./services";
import { Router } from "vue-router";

declare module "*.vue" {
  import type { DefineComponent } from "vue";
  // eslint-disable-next-line @typescript-eslint/ban-types, @typescript-eslint/no-explicit-any
  const component: DefineComponent<{}, {}, any>;
  export default component;
}

interface ImportMetaEnv {
  readonly VITE_API_TYPE: "local" | "graphql" | "jsonapi" | "grpc" | "mock";
  readonly VITE_API_URL: string;
  // more env variables...
}

// eslint-disable-next-line no-unused-vars
interface ImportMeta {
  readonly env: ImportMetaEnv;
}

declare module "pinia" {
  export interface PiniaCustomProperties {
    readonly authService: AuthService;
    readonly router: Router;
  }
}
