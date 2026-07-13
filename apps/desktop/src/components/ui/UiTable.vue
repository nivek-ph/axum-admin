<template>
  <div>
    <span class="hidden">
      <slot />
    </span>
    <div class="relative overflow-x-auto rounded-md border border-stone-200 bg-white">
      <div
        v-if="loading && rows.length > 0"
        class="absolute inset-x-0 top-0 z-10 h-0.5 overflow-hidden bg-stone-100"
      >
        <div class="h-full w-1/3 animate-pulse rounded-full bg-zinc-300" />
      </div>
      <table v-bind="$attrs" class="w-full border-collapse text-left text-sm">
        <thead class="bg-stone-50 text-xs font-medium text-zinc-600">
          <tr>
            <th
              v-for="column in columns"
              :key="String(column.id)"
              class="border-b border-stone-200 px-4 py-3"
              :style="{ width: normalizeSize(column.width), minWidth: normalizeSize(column.minWidth) }"
            >
              <input
                v-if="column.type === 'selection'"
                type="checkbox"
                :checked="rows.length > 0 && selectedRows.length === rows.length"
                @change="setAllSelected(($event.target as HTMLInputElement).checked)"
              />
              <span v-else>{{ column.label ? t(column.label) : '' }}</span>
            </th>
          </tr>
        </thead>
        <tbody :class="loading && rows.length > 0 ? 'pointer-events-none opacity-60' : ''">
          <tr v-if="loading && rows.length === 0">
            <td :colspan="Math.max(columns.length, 1)" class="px-4 py-8 text-center text-zinc-500">
              {{ t('Loading') }}...
            </td>
          </tr>
          <tr v-else-if="!rows.length">
            <td :colspan="Math.max(columns.length, 1)" class="px-4 py-8 text-center text-zinc-500">
              {{ t('No data') }}
            </td>
          </tr>
          <tr
            v-for="{ row, level } in rows"
            v-else
            :key="rowKey(row, level)"
            class="border-b border-stone-100 last:border-b-0 hover:bg-stone-50/70"
            @click="$emit('current-change', row)"
          >
            <td
              v-for="(column, index) in columns"
              :key="String(column.id)"
              class="px-4 py-3 text-zinc-800"
              :style="{
                width: normalizeSize(column.width),
                minWidth: normalizeSize(column.minWidth),
                paddingLeft: isTreeTable && index === treeColumnIndex && level ? `${level * 18 + 16}px` : undefined
              }"
            >
              <input
                v-if="column.type === 'selection'"
                type="checkbox"
                :checked="isSelected(row)"
                @click.stop
                @change="setSelected(row, ($event.target as HTMLInputElement).checked)"
              />
              <div v-else-if="isTreeTable && index === treeColumnIndex" class="tree-cell flex items-center gap-2">
                <button
                  v-if="hasChildren(row)"
                  :data-test="`tree-toggle-${rowKey(row, level)}`"
                  type="button"
                  class="grid h-7 w-7 shrink-0 place-items-center rounded text-zinc-500 hover:bg-stone-100 hover:text-zinc-900"
                  :aria-label="t(isExpanded(row, level) ? 'Collapse' : 'Expand')"
                  :aria-expanded="isExpanded(row, level)"
                  @click.stop="toggleExpanded(row, level)"
                >
                  <svg
                    class="h-5 w-5 transition-transform"
                    :class="isExpanded(row, level) ? 'rotate-90' : ''"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2.5"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    aria-hidden="true"
                  >
                    <path d="m9 18 6-6-6-6" />
                  </svg>
                </button>
                <span v-else class="tree-cell-spacer inline-block h-7 w-7 shrink-0" />
                <component v-if="column.slots.default" :is="column.slots.default" :row="row" />
                <span v-else>{{ valueAt(row, column.prop) }}</span>
              </div>
              <component v-else-if="column.slots.default" :is="column.slots.default" :row="row" />
              <span v-else>{{ valueAt(row, column.prop) }}</span>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, provide, ref, watch } from 'vue'
import { t } from '@/i18n'
import { uiTableKey, type UiTableColumnDef, type UiTableContext } from './UiTableColumn.vue'

defineOptions({ inheritAttrs: false })

const props = withDefaults(
  defineProps<{
    data?: Record<string, unknown>[]
    loading?: boolean
    defaultExpandAll?: boolean
  }>(),
  {
    data: () => [],
    defaultExpandAll: false
  }
)

const emit = defineEmits<{
  'current-change': [row: Record<string, unknown>]
  'selection-change': [rows: Record<string, unknown>[]]
}>()

const columns = ref<UiTableColumnDef[]>([])
const selectedRows = ref<Record<string, unknown>[]>([])
const expandedKeys = ref(new Set<string>())

provide<UiTableContext>(uiTableKey, {
  register(column) {
    columns.value = [...columns.value.filter((item) => item.id !== column.id), column]
  },
  unregister(id) {
    columns.value = columns.value.filter((item) => item.id !== id)
  }
})

const treeColumnIndex = computed(() => columns.value.findIndex((column) => column.type !== 'selection'))
const isTreeTable = computed(() => containsTreeRows(props.data))
const rows = computed(() => flattenRows(props.data))

watch(
  () => props.data,
  () => {
    selectedRows.value = []
    emit('selection-change', [])
  }
)

watch(
  [() => props.data, () => props.defaultExpandAll],
  () => {
    expandedKeys.value = props.defaultExpandAll
      ? new Set(collectExpandableKeys(props.data))
      : new Set()
  },
  { immediate: true }
)

function flattenRows(input: Record<string, unknown>[], level = 0): Array<{ row: Record<string, unknown>; level: number }> {
  return input.flatMap((row) => {
    const children = Array.isArray(row.children) ? (row.children as Record<string, unknown>[]) : []
    return isExpanded(row, level)
      ? [{ row, level }, ...flattenRows(children, level + 1)]
      : [{ row, level }]
  })
}

function collectExpandableKeys(input: Record<string, unknown>[], level = 0): string[] {
  return input.flatMap((row) => {
    const children = Array.isArray(row.children) ? (row.children as Record<string, unknown>[]) : []
    return children.length > 0
      ? [rowKey(row, level), ...collectExpandableKeys(children, level + 1)]
      : []
  })
}

function containsTreeRows(input: Record<string, unknown>[]): boolean {
  return input.some((row) => {
    const children = Array.isArray(row.children) ? (row.children as Record<string, unknown>[]) : []
    return children.length > 0 || containsTreeRows(children)
  })
}

function normalizeSize(value?: string | number) {
  if (value === undefined) return undefined
  return typeof value === 'number' ? `${value}px` : value
}

function valueAt(row: Record<string, unknown>, prop?: string) {
  return prop ? String(row[prop] ?? '') : ''
}

function rowKey(row: Record<string, unknown>, level: number) {
  return String(row.id ?? row.path ?? row.name ?? `${level}-${props.data.indexOf(row)}`)
}

function hasChildren(row: Record<string, unknown>) {
  return Array.isArray(row.children) && row.children.length > 0
}

function isExpanded(row: Record<string, unknown>, level: number) {
  return expandedKeys.value.has(rowKey(row, level))
}

function toggleExpanded(row: Record<string, unknown>, level: number) {
  const key = rowKey(row, level)
  const next = new Set(expandedKeys.value)
  if (next.has(key)) next.delete(key)
  else next.add(key)
  expandedKeys.value = next
}

function isSelected(row: Record<string, unknown>) {
  return selectedRows.value.includes(row)
}

function setSelected(row: Record<string, unknown>, checked: boolean) {
  selectedRows.value = checked
    ? [...selectedRows.value, row]
    : selectedRows.value.filter((item) => item !== row)
  emit('selection-change', selectedRows.value)
}

function setAllSelected(checked: boolean) {
  selectedRows.value = checked ? rows.value.map((item) => item.row) : []
  emit('selection-change', selectedRows.value)
}
</script>
