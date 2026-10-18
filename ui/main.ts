import { createApp } from 'vue';
import App from './app/App.vue';
import { router } from './app/router';
import './styles/tokens.css';
import './styles/dracula.css';
import './styles/global.css';

createApp(App).use(router).mount('#app');
