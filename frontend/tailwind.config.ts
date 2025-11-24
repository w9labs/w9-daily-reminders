import type { Config } from 'tailwindcss'

const config: Config = {
  content: ['./app/**/*.{ts,tsx}', './components/**/*.{ts,tsx}', './lib/**/*.{ts,tsx}'],
  theme: {
    extend: {
      colors: {
        bg: '#000000',
        fg: '#ffffff'
      },
      fontFamily: {
        console: ['\"Courier New\"', 'Courier', 'monospace']
      },
      borderWidth: {
        3: '3px'
      }
    }
  },
  plugins: []
}

export default config
