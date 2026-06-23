import '@testing-library/jest-dom/vitest';
import { cleanup } from '@testing-library/react';
import { afterEach } from 'vitest';
import { baseLocale, overwriteGetLocale } from '../src/paraglide/runtime';

// Pin the locale so paraglide message getters don't resolve a strategy (cookie/localStorage)
// that isn't available in the jsdom test environment.
overwriteGetLocale(() => baseLocale);

afterEach(() => {
  cleanup();
});
