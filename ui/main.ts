import { createApp } from 'vue';
import App from './app/App.vue';
import { router } from './app/router';
import { initializeTheme } from './app/theme';
import './styles/tokens.css';
import './styles/dracula.css';
import './styles/light.css';
import './styles/global.css';

initializeTheme();
createApp(App).use(router).mount('#app');
