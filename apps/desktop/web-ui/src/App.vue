<script setup lang="ts">
import { computed } from 'vue'
import { RouterView, useRoute } from 'vue-router'
import AppSidebar from '@/components/layout/AppSidebar.vue'
import ConnectionBanner from '@/components/layout/ConnectionBanner.vue'
import ToastStack from '@/components/ui/ToastStack.vue'
import ConfirmDialog from '@/components/ui/ConfirmDialog.vue'
import { useConfirm } from '@/composables/useConfirm'

const route = useRoute()
const { confirmRequest, settle } = useConfirm()

const title = computed(() => (typeof route.meta.title === 'string' ? route.meta.title : 'QingLink'))
const description = computed(() =>
  typeof route.meta.description === 'string' ? route.meta.description : undefined,
)
</script>

<template>
  <div class="shell">
    <AppSidebar />

    <div class="shell__column">
      <ConnectionBanner />
      <main class="shell__content">
        <header class="page-head">
          <h1 class="page-head__title">{{ title }}</h1>
          <p v-if="description" class="page-head__desc">{{ description }}</p>
        </header>
        <RouterView />
      </main>
    </div>

    <ToastStack />
    <ConfirmDialog
      v-if="confirmRequest"
      :open="true"
      :title="confirmRequest.title"
      :message="confirmRequest.message"
      :detail="confirmRequest.detail"
      :confirm-text="confirmRequest.confirmText"
      :cancel-text="confirmRequest.cancelText"
      :tone="confirmRequest.tone"
      @confirm="settle(true)"
      @cancel="settle(false)"
    />
  </div>
</template>

<style scoped>
.shell {
  display: grid;
  grid-template-columns: var(--sidebar-width) minmax(0, 1fr);
  min-height: 100vh;
  min-width: 720px;
}

.shell__column {
  display: flex;
  flex-direction: column;
  min-width: 0;
  background: var(--surface-0);
}

.shell__content {
  flex: 1 1 auto;
  width: 100%;
  max-width: var(--content-max);
  margin: 0 auto;
  padding: var(--space-8) var(--space-8) var(--space-8);
  display: flex;
  flex-direction: column;
  gap: var(--space-6);
  min-width: 0;
}

.page-head {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
}

.page-head__title {
  font-size: var(--font-2xl);
  font-weight: var(--weight-semibold);
  letter-spacing: -0.03em;
  line-height: 1.2;
  color: var(--text-1);
}

.page-head__desc {
  max-width: 70ch;
  font-size: var(--font-md);
  color: var(--text-3);
}

@media (max-width: 1080px) {
  .shell__content {
    padding: var(--space-6) var(--space-5) var(--space-8);
  }
}
</style>
