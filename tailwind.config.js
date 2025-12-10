/** @type {import('tailwindcss').Config} */
export default {
  // 1. Enable system-based dark mode
  darkMode: 'media', 
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        electric: 'var(--color-electric)',
        glass: {
          border: 'var(--color-glass-border)', 
          surface: 'var(--color-glass-surface)', 
        },
        txt: {
          primary: 'var(--color-txt-primary)',
          secondary: 'var(--color-txt-secondary)', 
          tertiary: 'var(--color-txt-tertiary)',
        }
      },
      animation: {
        'slide-down': 'slideDown 0.2s cubic-bezier(0.16, 1, 0.3, 1)',
        'fade-in': 'fadeIn 0.2s ease-out',
        // Existing recording timer (59s)
        'countdown': 'countdown 59s linear forwards', 
        // 🚨 NEW: Auto-save timer (10s)
        'autosave': 'countdown 10s linear forwards',
      },
      keyframes: {
        slideDown: {
          '0%': { opacity: 0, transform: 'translateY(-8px) scale(0.98)' },
          '100%': { opacity: 1, transform: 'translateY(0) scale(1)' },
        },
        fadeIn: {
          '0%': { opacity: 0 },
          '100%': { opacity: 1 },
        },
        countdown: {
          '0%': { width: '100%' },
          '100%': { width: '0%' },
        }
      }
    },
  },
  plugins: [],
}
