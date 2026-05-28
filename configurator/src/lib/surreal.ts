import { Surreal, Value } from 'surrealdb'
import { markRaw } from 'vue'
import { inject, onUnmounted, provide, type InjectionKey, type Ref } from "vue";
import { useMutation } from "@tanstack/vue-query";

interface SurrealOptions {
  endpoint: string;
  client?: Surreal;
  params?: Parameters<Surreal["connect"]>[1];
  autoConnect?: boolean;
}

interface SurrealState {
  client: Surreal;
  isConnecting: Ref<boolean>;
  isSuccess: Ref<boolean>;
  isError: Ref<boolean>;
  error: Ref<unknown>;
  connect: () => Promise<true>;
  close: () => Promise<true>;
}

const SurrealKey: InjectionKey<SurrealState> = Symbol("surreal");

export function provideSurreal(options: SurrealOptions) {
  const instance = options.client ?? new Surreal({
    codecOptions: {
      valueDecodeVisitor: (value) => (value instanceof Value ? markRaw(value) : value),
    },
  });

  const { mutateAsync, isPending, isSuccess, isError, error, reset } = useMutation({
    mutationFn: () => instance.connect(options.endpoint, options.params),
  });

  if (options.autoConnect !== false) {
    mutateAsync();
  }

  onUnmounted(() => {
    reset();
    instance.close();
  });

  const state: SurrealState = {
    client: instance,
    isConnecting: isPending,
    isSuccess,
    isError,
    error,
    connect: () => mutateAsync(),
    close: () => instance.close(),
  };

  provide(SurrealKey, state);
  return state;
}

export function useSurreal(): SurrealState {
  const state = inject(SurrealKey);
  if (!state) throw new Error("useSurreal() requires provideSurreal() in a parent component");
  return state;
}

export function useSurrealClient(): Surreal {
  return useSurreal().client;
}
