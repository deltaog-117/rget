/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./resources/**/*.blade.php",
    "./resources/**/*.js",
    "./resources/**/*.vue",
    "./app/Features/**/*.php",
    "./app/Features/**/*.blade.php",
  ],
  theme: {
    extend: {
      colors: {
        'tux-bg': '#f6f8fa',
        'tux-surface': '#ffffff',
        'tux-border': '#e1e4e8',
        'tux-text': '#24292e',
        'tux-muted': '#57606a',
        'tux-green': '#2ea043',
        'tux-blue': '#0366d6',
        'tux-orange': '#d1861c',
        'tux-dark': '#1e1e1e',
        'tux-terminal-bg': '#1e1e1e',
        'tux-terminal-text': '#d4d4d4',
      },
      fontFamily: {
        sans: ['Inter', 'system-ui', 'sans-serif'],
        mono: ['JetBrains Mono', 'Fira Code', 'monospace'],
      },
    },
  },
  plugins: [],
}
