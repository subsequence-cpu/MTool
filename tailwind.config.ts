import type { Config } from 'tailwindcss';

export default {
  content: { relative: true, files: ['./ui/**/*.{html,vue,ts}'] },
  corePlugins: { preflight: false },
  theme: {
    extend: {
      colors: Object.fromEntries(
        [
          'background',
          'background-deep',
          'surface',
          'surface-hover',
          'selection',
          'foreground',
          'muted',
          'border',
          'accent',
          'accent-soft',
          'accent-foreground',
          'green',
          'cyan',
          'yellow',
          'orange',
          'red',
          'red-soft',
          'success-soft',
        ].map((name) => [name, `var(--${name})`]),
      ),
      fontFamily: { mono: ['var(--font-mono)'] },
      fontSize: { control: 'var(--control-font-size)' },
      borderRadius: {
        control: 'var(--radius-sm)',
        panel: 'var(--radius-md)',
        dialog: 'var(--radius-lg)',
      },
      spacing: {
        control: 'var(--control-height)',
        'control-small': 'var(--control-height-small)',
      },
      gap: { control: 'var(--control-gap)' },
      screens: {
        tablet: { max: '900px' },
        compact: { max: '800px' },
        narrow: { max: '650px' },
      },
    },
  },
} satisfies Config;
