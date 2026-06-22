FROM mcr.microsoft.com/playwright:v1.61.0-noble as base
WORKDIR /app
COPY package.json pnpm-lock.yaml ./
RUN npm i -g pnpm
RUN pnpm i --frozen-lockfile
FROM base as runner
COPY --from=base /app /app
WORKDIR /app
COPY . .
CMD pnpm playwright test
