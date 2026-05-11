import type { Config } from "tailwindcss";

const config: Config = {
  content: ["./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      fontFamily: {
        sans: ['var(--font-inter)', 'ui-sans-serif', 'system-ui'],
        mono: ['var(--font-jetbrains-mono)', 'ui-monospace', 'monospace'],
        serif: ['"Instrument Serif"', 'serif'],
      },
      colors: {
        ink: { 950:'#0a0a0b', 900:'#171718', 800:'#27272a', 700:'#3f3f46', 600:'#52525b', 500:'#71717a', 400:'#a1a1aa', 300:'#d4d4d8', 200:'#e4e4e7', 100:'#f4f4f5', 50:'#fafafa' },
        violet: { 50:'#f5f3ff', 100:'#ede9fe', 200:'#ddd6fe', 400:'#a78bfa', 500:'#8b5cf6', 600:'#7c3aed', 700:'#6d28d9', 900:'#4c1d95' },
        sol: { green:'#14F195', purple:'#9945FF' },
      },
      boxShadow: {
        card: '0 1px 0 0 rgba(0,0,0,0.04), 0 1px 3px 0 rgba(24,24,27,0.04)',
        pop: '0 8px 24px -8px rgba(24,24,27,0.18), 0 2px 6px -2px rgba(24,24,27,0.08)',
        'inset-line': 'inset 0 -1px 0 0 #e4e4e7',
      },
    },
  },
  plugins: [],
};
export default config;
