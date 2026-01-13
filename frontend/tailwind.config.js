/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        // Primary - Deep Teal
        primary: {
          DEFAULT: 'var(--color-primary)',
          hover: 'var(--color-primary-hover)',
          muted: 'var(--color-primary-muted)',
        },
        // Backgrounds
        bg: {
          base: 'var(--color-bg-base)',
          surface: 'var(--color-bg-surface)',
          elevated: 'var(--color-bg-elevated)',
          hover: 'var(--color-bg-hover)',
        },
        // Text
        text: {
          primary: 'var(--color-text-primary)',
          secondary: 'var(--color-text-secondary)',
          muted: 'var(--color-text-muted)',
        },
        // Accents
        semantic: 'var(--color-semantic)',
        fulltext: 'var(--color-fulltext)',
        success: 'var(--color-success)',
        warning: 'var(--color-warning)',
        error: 'var(--color-error)',
        // Borders
        border: {
          DEFAULT: 'var(--color-border)',
          focus: 'var(--color-border-focus)',
        },
      },
      fontFamily: {
        sans: ['Inter', 'system-ui', 'sans-serif'],
        mono: ['JetBrains Mono', 'ui-monospace', 'monospace'],
      },
    },
  },
  plugins: [],
}
