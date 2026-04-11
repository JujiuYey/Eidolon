<template>
  <ResizablePanelGroup direction="horizontal" class="h-full">
    <ResizablePanel :default-size="20" :min-size="15" :max-size="40">
      <div class="h-full border-r bg-muted/20">
        <slot name="files" />
      </div>
    </ResizablePanel>

    <ResizableHandle with-handle />

    <ResizablePanel :default-size="80" :min-size="40">
      <div class="h-full bg-background">
        <slot name="chat" />
      </div>
    </ResizablePanel>
  </ResizablePanelGroup>
</template>
