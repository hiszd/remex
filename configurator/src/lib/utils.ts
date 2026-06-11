import type { selectItem } from "@/components/MultiSelector.vue"
import type { Client, Group, Job } from "./model"

export const toSelectItem = (item: Group | Client | Job): selectItem => {
  if ('group_name' in item) {
    return {
      id: item.id,
      display_name: item.group_name,
      item,
    }
  } else if ('client_name' in item) {
    return {
      id: item.id,
      display_name: item.client_name,
      item,
    }
  }

  return {
    id: item.id,
    display_name: item.job_name,
    item,
  }
}

