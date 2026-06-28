<script setup lang="ts">
import { ref } from "vue"
import { useRouter } from "vue-router"
import { useMutation, useQueryClient } from "@tanstack/vue-query"
import { createGroup } from "@/lib/api"
import { useSurrealClient } from "@/lib/surreal"

const router = useRouter()
const queryClient = useQueryClient()
const client = useSurrealClient()

const form = ref({
  group_name: "",
})

const isSubmitting = ref(false)
const submitError = ref<string | null>(null)

const mutation = useMutation({
  mutationFn: (data: { group_name: string }) =>
    createGroup(client, {
      group_name: data.group_name,
      members: [],
    }),
  onSuccess: () => {
    queryClient.invalidateQueries({ queryKey: ["groups"] })
    router.push("/groups")
  },
  onError: (err: Error) => {
    submitError.value = err.message || "Failed to create group"
    isSubmitting.value = false
  },
})

const handleSubmit = async () => {
  isSubmitting.value = true
  submitError.value = null
  mutation.mutate({
    group_name: form.value.group_name,
  })
}
</script>

<template>
  <div class="page" style="max-width: 800px;">
    <router-link to="/groups" class="back-link">← Back to Groups</router-link>
    <h1 class="page-title" style="margin-bottom: 1.5rem;">Create New Group</h1>

    <form @submit.prevent="handleSubmit" class="card">
      <div v-if="submitError" class="error-banner">
        {{ submitError }}
      </div>

      <div class="form-group">
        <label for="group_name" class="form-label">Group Name</label>
        <input id="group_name" v-model="form.group_name" type="text" class="form-input" placeholder="e.g. Production Servers" required />
      </div>

      <div class="form-actions">
        <button type="button" @click="router.push('/groups')" class="btn-ghost" :disabled="isSubmitting">Cancel</button>
        <button type="submit" class="btn-primary" :disabled="isSubmitting">
          {{ isSubmitting ? "Creating..." : "Create Group" }}
        </button>
      </div>
    </form>
  </div>
</template>

<style lang="scss" scoped>
.form-group {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.error-banner {
  padding: 1rem;
  background-color: var(--accent-50);
  border: 1px solid var(--accent-400);
  color: var(--accent-900);
  border-radius: 0.5rem;
  font-size: 0.9rem;
}

.form-actions {
  display: flex;
  justify-content: flex-end;
  gap: 1rem;
  margin-top: 1rem;
  padding-top: 1.5rem;
  border-top: 1px solid var(--background-400);
}
</style>
