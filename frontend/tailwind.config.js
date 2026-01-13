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
          DEFAULT: '#2dd4bf',
          hover: '#5eead4',
          muted: '#134e4a',
        },
        // Backgrounds - Rich dark with subtle blue undertone
        bg: {
          base: '#0f0f14',
          surface: '#1a1a24',
          elevated: '#242430',
          hover: '#2e2e3a',
        },
        // Text
        text: {
          primary: '#f0f0f5',
          secondary: '#9898a8',
          muted: '#606070',
        },
        // Accents
        semantic: '#a78bfa',  // violet - semantic search
        fulltext: '#38bdf8',  // sky - fulltext search
        success: '#4ade80',
        warning: '#fbbf24',
        error: '#f87171',
        // Borders
        border: {
          DEFAULT: '#2e2e3a',
          focus: '#2dd4bf',
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
