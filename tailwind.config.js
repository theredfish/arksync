/** @type {import('tailwindcss').Config} */
module.exports = {
  content: {
    files: ["*.html", "./src/**/*.rs"],
    transform: {
      rs: (content) => content.replace(/(?:^|\s)class:/g, ' '),
    },
  },
  theme: {
    extend: {
      colors: {
        darcula: {
          gray: "#191919",
          black: "#0a0a0a",
          "purple-50": "#39344a",
          "purple-100": "#55507e",
          "purple-200": "#52406d",
          "purple-300": "#44355B",
          "purple-400": "#2c2432",
          "purple-500": "#251B49",
          "purple-600": "#0f172a",
          "yellow-100": "#ffde91",
          "yellow-200": "#fcc661",
          "yellow-300": "#FFBC1F",
          "yellow-400": "#ff7e6e",
          "yellow-500": "#ff7170",
          "yellow-600": "#F26B3A",
        },
      },
      gridRow: {
        'span-7': 'span 7 / span 7',
        'span-8': 'span 8 / span 8',
        'span-9': 'span 9 / span 9',
        'span-10': 'span 10 / span 10',
        'span-11': 'span 11 / span 11',
        'span-12': 'span 12 / span 12',
      },
      fontFamily: {
        hubot: ["var(--font-hubot-sans)"],
        mona: ["var(--font-mona-sans)"],
      },
    },
  },
  plugins: [],
}
