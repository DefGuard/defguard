FROM mcr.microsoft.com/playwright:v1.32.0-focal as base
WORKDIR /app
COPY package.json .
COPY pnpm-lock.yaml .
RUN npm i -g pnpm
RUN pnpm i --frozen-lockfile
FROM base as runner
COPY --from=base /app /app
WORKDIR /app
COPY . .
CMD pnpm playwright test
