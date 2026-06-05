/** @type {import('tailwindcss').Config} */
export default {
  darkMode: 'class',
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        // Core surfaces
        background: 'var(--background)',
        card: {
          DEFAULT: 'var(--card)',
          hover: 'var(--card-hover)',
          active: 'var(--card-active)',
        },
        border: {
          DEFAULT: 'var(--border)',
          light: 'var(--border-light)',
        },
        // Primary / accent
        primary: {
          DEFAULT: 'var(--primary)',
          hover: 'var(--primary-hover)',
          foreground: 'var(--primary-foreground)',
          subtle: 'var(--primary-subtle)',
        },
        // Text hierarchy
        foreground: {
          DEFAULT: 'var(--foreground)',
          dim: 'var(--foreground-dim)',
          muted: 'var(--foreground-muted)',
        },
        // Muted surfaces
        muted: {
          DEFAULT: 'var(--muted)',
          foreground: 'var(--muted-foreground)',
        },
        // Semantic
        destructive: {
          DEFAULT: 'var(--destructive)',
          foreground: 'var(--destructive-foreground)',
          subtle: 'var(--destructive-subtle)',
        },
        success: {
          DEFAULT: 'var(--success)',
          subtle: 'var(--success-subtle)',
        },
        warning: {
          DEFAULT: 'var(--warning)',
          subtle: 'var(--warning-subtle)',
        },
        info: {
          DEFAULT: 'var(--info)',
          subtle: 'var(--info-subtle)',
        },
        // Input fields
        input: {
          bg: 'var(--input-bg)',
          border: 'var(--input-border)',
          focus: 'var(--input-focus)',
          placeholder: 'var(--input-placeholder)',
        },
        // Chat
        chat: {
          user: {
            bg: 'var(--chat-user-bg)',
            text: 'var(--chat-user-text)',
          },
          assistant: {
            bg: 'var(--chat-assistant-bg)',
            text: 'var(--chat-assistant-text)',
          },
        },
        // Sidebar
        sidebar: {
          bg: 'var(--sidebar-bg)',
          hover: 'var(--sidebar-hover)',
          active: 'var(--sidebar-active)',
          indicator: 'var(--sidebar-active-indicator)',
          icon: 'var(--sidebar-icon)',
          'icon-active': 'var(--sidebar-icon-active)',
        },
        // Status bar
        statusbar: {
          bg: 'var(--statusbar-bg)',
          border: 'var(--statusbar-border)',
        },
        // Badge
        badge: {
          bg: 'var(--badge-bg)',
          text: 'var(--badge-text)',
        },
        // Misc
        ring: 'var(--ring)',
      },
      borderRadius: {
        DEFAULT: 'var(--radius)',
      },
    },
  },
  plugins: [],
}
