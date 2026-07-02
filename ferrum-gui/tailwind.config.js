/** @type {import('tailwindcss').Config} */
module.exports = {
  content: ['./src/**/*.{js,ts,jsx,tsx}'],
  theme: {
    extend: {
      colors: {
        space: {
          950: '#050810',
          900: '#0a0f1a',
          800: '#111827',
          700: '#1a2332',
        },
        cyan: {
          neon: '#00e5ff',
          glow: '#00bcd4',
        },
        rust: {
          orange: '#f74c00',
          ember: '#ff6b35',
        },
      },
      fontFamily: {
        mono: ['JetBrains Mono', 'Fira Code', 'monospace'],
        display: ['Orbitron', 'sans-serif'],
      },
      boxShadow: {
        neon: '0 0 20px rgba(0, 229, 255, 0.3)',
        rust: '0 0 20px rgba(247, 76, 0, 0.4)',
      },
    },
  },
  plugins: [],
};
