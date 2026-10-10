import { writable } from 'svelte/store';

// Light or dark UI. The choice is saved in localStorage and set as
// data-theme on <html>, which switches the CSS variables in app.css.
export type Theme = 'light' | 'dark';

const KEY = 'weft-theme';

function savedTheme(): Theme {
  return localStorage.getItem(KEY) === 'dark' ? 'dark' : 'light';
}

export const theme = writable<Theme>(savedTheme());

theme.subscribe((t) => {
  document.documentElement.dataset.theme = t;
  localStorage.setItem(KEY, t);
});

export function toggleTheme() {
  theme.update((t) => (t === 'dark' ? 'light' : 'dark'));
}
