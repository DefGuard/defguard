// vite.config.mts
import "file:///F:/work/defguard/web/node_modules/.pnpm/dotenv@16.4.7/node_modules/dotenv/config.js";
import react from "file:///F:/work/defguard/web/node_modules/.pnpm/@vitejs+plugin-react-swc@3.5.0_vite@5.0.12_@types+node@20.11.7_sass@1.70.0_terser@5.27.0_/node_modules/@vitejs/plugin-react-swc/index.mjs";
import autoprefixer from "file:///F:/work/defguard/web/node_modules/.pnpm/autoprefixer@10.4.17_postcss@8.4.33/node_modules/autoprefixer/lib/autoprefixer.js";
import * as path from "path";
import { defineConfig } from "file:///F:/work/defguard/web/node_modules/.pnpm/vite@5.0.12_@types+node@20.11.7_sass@1.70.0_terser@5.27.0/node_modules/vite/dist/node/index.js";
var __vite_injected_original_dirname = "F:\\work\\defguard\\web";
var vite_config_default = ({}) => {
  let proxyTarget = "http://127.0.0.1:8000/";
  const envProxyTarget = process.env.PROXY_TARGET;
  if (envProxyTarget && envProxyTarget.length > 0) {
    proxyTarget = envProxyTarget;
  }
  return defineConfig({
    clearScreen: false,
    plugins: [react()],
    server: {
      strictPort: false,
      port: 3e3,
      proxy: {
        "/api": {
          target: proxyTarget,
          changeOrigin: true
        },
        "/.well-known": {
          target: proxyTarget,
          changeOrigin: true
        },
        "/svg": {
          target: proxyTarget,
          changeOrigin: true
        }
      },
      fs: {
        allow: ["."]
      }
    },
    envPrefix: ["VITE_"],
    assetsInclude: ["./src/shared/assets/**/*"],
    resolve: {
      alias: {
        "@scss": path.resolve(__vite_injected_original_dirname, "./src/shared/scss"),
        "@scssutils": path.resolve(__vite_injected_original_dirname, "./src/shared/scss/global")
      }
    },
    build: {
      chunkSizeWarningLimit: 1e4,
      rollupOptions: {
        logLevel: "silent",
        // eslint-disable-next-line @typescript-eslint/no-unused-vars
        onwarn: (_warning, _warn) => {
          return;
        }
      }
    },
    css: {
      preprocessorOptions: {
        scss: {
          additionalData: `@use "@scssutils" as *;
`
        }
      },
      postcss: {
        plugins: [autoprefixer]
      }
    }
  });
};
export {
  vite_config_default as default
};
//# sourceMappingURL=data:application/json;base64,ewogICJ2ZXJzaW9uIjogMywKICAic291cmNlcyI6IFsidml0ZS5jb25maWcubXRzIl0sCiAgInNvdXJjZXNDb250ZW50IjogWyJjb25zdCBfX3ZpdGVfaW5qZWN0ZWRfb3JpZ2luYWxfZGlybmFtZSA9IFwiRjpcXFxcd29ya1xcXFxkZWZndWFyZFxcXFx3ZWJcIjtjb25zdCBfX3ZpdGVfaW5qZWN0ZWRfb3JpZ2luYWxfZmlsZW5hbWUgPSBcIkY6XFxcXHdvcmtcXFxcZGVmZ3VhcmRcXFxcd2ViXFxcXHZpdGUuY29uZmlnLm10c1wiO2NvbnN0IF9fdml0ZV9pbmplY3RlZF9vcmlnaW5hbF9pbXBvcnRfbWV0YV91cmwgPSBcImZpbGU6Ly8vRjovd29yay9kZWZndWFyZC93ZWIvdml0ZS5jb25maWcubXRzXCI7aW1wb3J0ICdkb3RlbnYvY29uZmlnJztcblxuaW1wb3J0IHJlYWN0IGZyb20gJ0B2aXRlanMvcGx1Z2luLXJlYWN0LXN3Yyc7XG5pbXBvcnQgYXV0b3ByZWZpeGVyIGZyb20gJ2F1dG9wcmVmaXhlcic7XG5pbXBvcnQgKiBhcyBwYXRoIGZyb20gJ3BhdGgnO1xuaW1wb3J0IHsgZGVmaW5lQ29uZmlnIH0gZnJvbSAndml0ZSc7XG5cbi8vIGVzbGludC1kaXNhYmxlLW5leHQtbGluZSBuby1lbXB0eS1wYXR0ZXJuXG5leHBvcnQgZGVmYXVsdCAoe30pID0+IHtcbiAgbGV0IHByb3h5VGFyZ2V0ID0gJ2h0dHA6Ly8xMjcuMC4wLjE6ODAwMC8nO1xuICBjb25zdCBlbnZQcm94eVRhcmdldCA9IHByb2Nlc3MuZW52LlBST1hZX1RBUkdFVDtcblxuICBpZiAoZW52UHJveHlUYXJnZXQgJiYgZW52UHJveHlUYXJnZXQubGVuZ3RoID4gMCkge1xuICAgIHByb3h5VGFyZ2V0ID0gZW52UHJveHlUYXJnZXQ7XG4gIH1cblxuICByZXR1cm4gZGVmaW5lQ29uZmlnKHtcbiAgICBjbGVhclNjcmVlbjogZmFsc2UsXG4gICAgcGx1Z2luczogW3JlYWN0KCldLFxuICAgIHNlcnZlcjoge1xuICAgICAgc3RyaWN0UG9ydDogZmFsc2UsXG4gICAgICBwb3J0OiAzMDAwLFxuICAgICAgcHJveHk6IHtcbiAgICAgICAgJy9hcGknOiB7XG4gICAgICAgICAgdGFyZ2V0OiBwcm94eVRhcmdldCxcbiAgICAgICAgICBjaGFuZ2VPcmlnaW46IHRydWUsXG4gICAgICAgIH0sXG4gICAgICAgICcvLndlbGwta25vd24nOiB7XG4gICAgICAgICAgdGFyZ2V0OiBwcm94eVRhcmdldCxcbiAgICAgICAgICBjaGFuZ2VPcmlnaW46IHRydWUsXG4gICAgICAgIH0sXG4gICAgICAgICcvc3ZnJzoge1xuICAgICAgICAgIHRhcmdldDogcHJveHlUYXJnZXQsXG4gICAgICAgICAgY2hhbmdlT3JpZ2luOiB0cnVlLFxuICAgICAgICB9LFxuICAgICAgfSxcbiAgICAgIGZzOiB7XG4gICAgICAgIGFsbG93OiBbJy4nXSxcbiAgICAgIH0sXG4gICAgfSxcbiAgICBlbnZQcmVmaXg6IFsnVklURV8nXSxcbiAgICBhc3NldHNJbmNsdWRlOiBbJy4vc3JjL3NoYXJlZC9hc3NldHMvKiovKiddLFxuICAgIHJlc29sdmU6IHtcbiAgICAgIGFsaWFzOiB7XG4gICAgICAgICdAc2Nzcyc6IHBhdGgucmVzb2x2ZShfX2Rpcm5hbWUsICcuL3NyYy9zaGFyZWQvc2NzcycpLFxuICAgICAgICAnQHNjc3N1dGlscyc6IHBhdGgucmVzb2x2ZShfX2Rpcm5hbWUsICcuL3NyYy9zaGFyZWQvc2Nzcy9nbG9iYWwnKSxcbiAgICAgIH0sXG4gICAgfSxcbiAgICBidWlsZDoge1xuICAgICAgY2h1bmtTaXplV2FybmluZ0xpbWl0OiAxMDAwMCxcbiAgICAgIHJvbGx1cE9wdGlvbnM6IHtcbiAgICAgICAgbG9nTGV2ZWw6ICdzaWxlbnQnLFxuICAgICAgICAvLyBlc2xpbnQtZGlzYWJsZS1uZXh0LWxpbmUgQHR5cGVzY3JpcHQtZXNsaW50L25vLXVudXNlZC12YXJzXG4gICAgICAgIG9ud2FybjogKF93YXJuaW5nLCBfd2FybikgPT4ge1xuICAgICAgICAgIHJldHVybjtcbiAgICAgICAgfSxcbiAgICAgIH0sXG4gICAgfSxcbiAgICBjc3M6IHtcbiAgICAgIHByZXByb2Nlc3Nvck9wdGlvbnM6IHtcbiAgICAgICAgc2Nzczoge1xuICAgICAgICAgIGFkZGl0aW9uYWxEYXRhOiBgQHVzZSBcIkBzY3NzdXRpbHNcIiBhcyAqO1xcbmAsXG4gICAgICAgIH0sXG4gICAgICB9LFxuICAgICAgcG9zdGNzczoge1xuICAgICAgICBwbHVnaW5zOiBbYXV0b3ByZWZpeGVyXSxcbiAgICAgIH0sXG4gICAgfSxcbiAgfSk7XG59O1xuIl0sCiAgIm1hcHBpbmdzIjogIjtBQUF3UCxPQUFPO0FBRS9QLE9BQU8sV0FBVztBQUNsQixPQUFPLGtCQUFrQjtBQUN6QixZQUFZLFVBQVU7QUFDdEIsU0FBUyxvQkFBb0I7QUFMN0IsSUFBTSxtQ0FBbUM7QUFRekMsSUFBTyxzQkFBUSxDQUFDLENBQUMsTUFBTTtBQUNyQixNQUFJLGNBQWM7QUFDbEIsUUFBTSxpQkFBaUIsUUFBUSxJQUFJO0FBRW5DLE1BQUksa0JBQWtCLGVBQWUsU0FBUyxHQUFHO0FBQy9DLGtCQUFjO0FBQUEsRUFDaEI7QUFFQSxTQUFPLGFBQWE7QUFBQSxJQUNsQixhQUFhO0FBQUEsSUFDYixTQUFTLENBQUMsTUFBTSxDQUFDO0FBQUEsSUFDakIsUUFBUTtBQUFBLE1BQ04sWUFBWTtBQUFBLE1BQ1osTUFBTTtBQUFBLE1BQ04sT0FBTztBQUFBLFFBQ0wsUUFBUTtBQUFBLFVBQ04sUUFBUTtBQUFBLFVBQ1IsY0FBYztBQUFBLFFBQ2hCO0FBQUEsUUFDQSxnQkFBZ0I7QUFBQSxVQUNkLFFBQVE7QUFBQSxVQUNSLGNBQWM7QUFBQSxRQUNoQjtBQUFBLFFBQ0EsUUFBUTtBQUFBLFVBQ04sUUFBUTtBQUFBLFVBQ1IsY0FBYztBQUFBLFFBQ2hCO0FBQUEsTUFDRjtBQUFBLE1BQ0EsSUFBSTtBQUFBLFFBQ0YsT0FBTyxDQUFDLEdBQUc7QUFBQSxNQUNiO0FBQUEsSUFDRjtBQUFBLElBQ0EsV0FBVyxDQUFDLE9BQU87QUFBQSxJQUNuQixlQUFlLENBQUMsMEJBQTBCO0FBQUEsSUFDMUMsU0FBUztBQUFBLE1BQ1AsT0FBTztBQUFBLFFBQ0wsU0FBYyxhQUFRLGtDQUFXLG1CQUFtQjtBQUFBLFFBQ3BELGNBQW1CLGFBQVEsa0NBQVcsMEJBQTBCO0FBQUEsTUFDbEU7QUFBQSxJQUNGO0FBQUEsSUFDQSxPQUFPO0FBQUEsTUFDTCx1QkFBdUI7QUFBQSxNQUN2QixlQUFlO0FBQUEsUUFDYixVQUFVO0FBQUE7QUFBQSxRQUVWLFFBQVEsQ0FBQyxVQUFVLFVBQVU7QUFDM0I7QUFBQSxRQUNGO0FBQUEsTUFDRjtBQUFBLElBQ0Y7QUFBQSxJQUNBLEtBQUs7QUFBQSxNQUNILHFCQUFxQjtBQUFBLFFBQ25CLE1BQU07QUFBQSxVQUNKLGdCQUFnQjtBQUFBO0FBQUEsUUFDbEI7QUFBQSxNQUNGO0FBQUEsTUFDQSxTQUFTO0FBQUEsUUFDUCxTQUFTLENBQUMsWUFBWTtBQUFBLE1BQ3hCO0FBQUEsSUFDRjtBQUFBLEVBQ0YsQ0FBQztBQUNIOyIsCiAgIm5hbWVzIjogW10KfQo=
