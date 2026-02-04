// @ts-check
import { defineConfig } from 'astro/config';

// https://astro.build/config
export default defineConfig({
  site: 'https://hazelnut.ricardodantas.me',
  base: '/',
  build: {
    assets: 'assets'
  }
});
