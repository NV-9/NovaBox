/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './src/**/*.{js,ts,jsx,tsx}'],
  theme: {
    extend: {
      colors: {
        // NovaBox brand palette
        nova: {
          50:  '#f0f4ff',
          100: '#dce6ff',
          200: '#b9ceff',
          300: '#8aacff',
          400: '#5680ff',
          500: '#2855ff',
          600: '#1233f5',
          700: '#0e22e0',
          800: '#111eb4',
          900: '#141f8e',
          950: '#0a1056',
        },
        dark: {
          50:  '#f6f7f9',
          100: '#eceef2',
          200: '#d4d9e3',
          300: '#afb9ca',
          400: '#8494ac',
          500: '#647592',
          600: '#505d78',
          700: '#414b62',
          800: '#374053',
          900: '#313847',
          950: '#141720',
          bg:  '#0d0f17',
          card:'#141720',
          border: '#1e2233',
        },
      },
      fontFamily: {
        mono: ['JetBrains Mono', 'Fira Code', 'Cascadia Code', 'monospace'],
      },
    },
  },
  plugins: [],
}
