<template>
  <div class="admin-page">
    <section class="page-hero">
      <div class="page-hero-main">
        <span class="page-hero-kicker">{{ $t('Access catalog') }}</span>
        <h2 class="page-hero-title">{{ $t('Menus and permissions') }}</h2>
        <p class="page-hero-subtitle">{{ $t('Definitions are managed by database migrations and are read-only here.') }}</p>
      </div>
    </section>

    <div class="page-panel">
      <UiTable :data="menus" :loading="loading" row-key="id">
        <UiTableColumn prop="meta.title" :label="$t('Name')" min-width="220">
          <template #default="{ row }"><strong>{{ row.meta?.title || row.name }}</strong></template>
        </UiTableColumn>
        <UiTableColumn prop="menuType" :label="$t('Type')" width="110" />
        <UiTableColumn prop="permission" :label="$t('Permission code')" min-width="220" />
        <UiTableColumn :label="$t('API bindings')" min-width="300">
          <template #default="{ row }">
            <div v-for="binding in row.apiBindings || []" :key="`${binding.method}:${binding.pathPattern}`" class="binding">
              <code>{{ binding.method }}</code> {{ binding.pathPattern }}
            </div>
            <span v-if="!row.apiBindings?.length">—</span>
          </template>
        </UiTableColumn>
      </UiTable>
    </div>
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { ElMessage } from '@/ui/feedback'
import { fetchMenuList, type MenuRecord } from '@/api/menus'
import { t } from '@/i18n'

const menus = ref<MenuRecord[]>([])
const loading = ref(false)

onMounted(async () => {
  loading.value = true
  try { menus.value = await fetchMenuList() }
  catch { ElMessage.error(t('Failed to load menus')) }
  finally { loading.value = false }
})
</script>

<style scoped>
.binding { line-height: 1.7; }
.binding code { margin-right: 0.4rem; }
</style>
