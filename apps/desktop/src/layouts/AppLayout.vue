<template>
  <div class="layout-shell">
    <div class="layout-frame">
      <AppSidebar />
      <main class="content-shell">
        <AppHeader />
        <section class="content-stage">
          <RouterView v-slot="{ Component, route }">
            <Transition name="content-fade" mode="out-in">
              <component :is="Component" :key="route.fullPath" />
            </Transition>
          </RouterView>
        </section>
      </main>
    </div>
  </div>
</template>

<script setup lang="ts">
import AppHeader from '@/components/AppHeader.vue'
import AppSidebar from '@/components/AppSidebar.vue'
</script>

<style scoped>
.layout-shell {
  min-height: 100vh;
  padding: 16px;
}

.layout-frame {
  display: grid;
  grid-template-columns: 286px minmax(0, 1fr);
  min-height: calc(100vh - 32px);
  border-radius: 24px;
  border: 1px solid var(--shell-border);
  background: var(--shell-bg);
  box-shadow: var(--shell-shadow);
  overflow: hidden;
}

.content-shell {
  display: flex;
  flex-direction: column;
  min-width: 0;
  background: #fafaf9;
}

.content-stage {
  flex: 1;
  min-width: 0;
  padding: 22px;
}

@media (max-width: 1180px) {
  .layout-frame {
    grid-template-columns: 220px minmax(0, 1fr);
  }
}

@media (max-width: 720px) {
  .layout-shell {
    padding: 12px;
  }

  .layout-frame {
    grid-template-columns: 1fr;
  }
}
</style>
