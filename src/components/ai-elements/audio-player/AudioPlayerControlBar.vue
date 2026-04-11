<script setup lang="ts">
import type { MediaControlBar } from 'media-chrome';
import { ButtonGroup } from '@/components/ui/button-group';

interface Props extends /* @vue-ignore */ Partial<MediaControlBar> {
  // Add any media-chrome control bar props if needed
}

defineProps<Props>();
</script>

<template>
  <MediaControlBar data-slot="audio-player-control-bar">
    <ButtonGroup orientation="horizontal">
      <slot />
    </ButtonGroup>
  </MediaControlBar>
</template>
