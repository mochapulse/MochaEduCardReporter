// Runs before the first paint: applies the saved theme and text size so the
// window never flashes the wrong look. main.tsx then applies the authoritative
// values read from the backend settings.
;(function () {
  try {
    const root = document.documentElement
    const theme = window.localStorage.getItem('mocha.theme')
    const size = window.localStorage.getItem('mocha.text-size')
    const dark =
      theme === 'oscuro' ||
      (theme === 'sistema' && window.matchMedia('(prefers-color-scheme: dark)').matches)
    root.dataset.theme = dark ? 'dark' : 'light'
    if (size === 'grande' || size === 'muy-grande') root.dataset.textSize = size
  } catch {
    // Storage unavailable: the defaults in index.html apply.
  }
})()
