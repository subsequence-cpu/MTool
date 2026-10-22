import tailwindcss from 'tailwindcss';
import autoprefixer from 'autoprefixer';

export default {
  plugins: [
    tailwindcss('./tailwind.config.ts'),
    autoprefixer({ overrideBrowserslist: ['Safari >= 16', 'Chrome >= 109'] }),
  ],
};
