<template>
  <div v-bind="wrapperAttrs()" class="relative min-h-10 min-w-0">
    <span class="pointer-events-none absolute left-3 top-1/2 z-10 -translate-y-1/2 text-[10px] font-semibold text-zinc-500">
      UTC
    </span>
    <input
      v-bind="inputAttrs()"
      type="datetime-local"
      :value="localValue"
      :aria-label="label ? t(label) : undefined"
      :disabled="disabled"
      class="min-h-10 w-full rounded-md border border-stone-300 bg-white py-2 pr-3 pl-12 text-sm text-zinc-900 outline-none transition focus-visible:border-stone-400 focus-visible:ring-2 focus-visible:ring-zinc-950/10 disabled:cursor-not-allowed disabled:bg-stone-100 disabled:text-zinc-500"
      @input="emitValue"
    />
  </div>
</template>

<script setup lang="ts">
import { computed, useAttrs } from 'vue'

import { t } from '@/i18n'

defineOptions({ inheritAttrs: false })

const props = defineProps<{
  modelValue?: string
  label?: string
  disabled?: boolean
}>()

const emit = defineEmits<{
  'update:modelValue': [value: string]
}>()

const attrs = useAttrs()

function wrapperAttrs() {
  return { class: attrs.class, style: attrs.style }
}

function inputAttrs() {
  return Object.fromEntries(
    Object.entries(attrs).filter(([key]) => key !== 'class' && key !== 'style')
  )
}

const localValue = computed(() => {
  if (!props.modelValue) return ''

  const date = new Date(props.modelValue)
  if (Number.isNaN(date.getTime())) return ''

  return date.toISOString().slice(0, 16)
})

function emitValue(event: Event) {
  const value = (event.target as HTMLInputElement).value
  emit('update:modelValue', value ? new Date(`${value}Z`).toISOString() : '')
}
</script>
