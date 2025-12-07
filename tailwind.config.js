/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        electric: '#FFC531', 
        glass: {
          border: 'rgba(255, 255, 255, 0.08)', 
          surface: 'rgba(20, 20, 20, 0.70)', 
        },
        txt: {
          primary: '#FFFFFF',
          secondary: 'rgba(255, 255, 255, 0.65)', 
          tertiary: 'rgba(255, 255, 255, 0.4)',
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
