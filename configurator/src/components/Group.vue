<script setup lang="ts">
import { formatDate } from "@/utils/date"
import { type Group } from "@/lib/model"
import { Hero } from "@/styles/color/hero"

const props = defineProps<{
  group: Group
}>()

const created = formatDate(props.group.created_at)
</script>

<template>
  <router-link :to="'/groups/' + group.id" class="hero-card-link">
    <div class="hero-card" :style="[Hero]">
      <div class="hero-header">
        <h1 class="hero-name">{{ group.group_name }}</h1>
        <p class="hero-id">{{ String(group.id) }}</p>
      </div>
      <div class="hero-details">
        <span class="detail">
          <p class="label">Created</p>
          <p class="value">{{ created }}</p>
        </span>
        <span class="detail">
          <p class="label">Members</p>
          <p class="value">{{ group.members?.length ?? 0 }} clients</p>
        </span>
      </div>
    </div>
  </router-link>
</template>

<style lang="scss" scoped>
@import "@/styles/hero-card.scss";

.hero-card-link {
  padding: 0;
}
</style>
