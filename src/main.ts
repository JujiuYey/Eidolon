import { createApp } from 'vue';
import { createPinia } from 'pinia';
import piniaPluginPersistedstate from 'pinia-plugin-persistedstate';
import App from './App.vue';
import router from './router';
import { useAppStore } from '@/stores/app';
import './index.css';

const app = createApp(App);
const pinia = createPinia();

// 注册持久化插件
pinia.use(piniaPluginPersistedstate);

async function bootstrap() {
  const appStore = useAppStore(pinia);
  await appStore.init();

  app
    .use(pinia)
    .use(router)
    .mount('#app');
}

void bootstrap();

// Vue应用挂载完成后隐藏初始加载动画
function hideInitialLoader() {
  const loader = document.getElementById('initial-loader');
  if (loader) {
    loader.classList.add('loader-hidden');
    // 动画结束后移除元素
    setTimeout(() => {
      loader.remove();
    }, 300);
  }
}

// 确保DOM完全加载后再隐藏加载器
if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', hideInitialLoader);
} else {
  // 如果DOM已经加载完成，延迟一点时间让用户看到加载动画
  setTimeout(hideInitialLoader, 500);
}
