FROM node:19.9-alpine as build

WORKDIR /app
COPY package.json .
COPY pnpm-lock.yaml .
COPY .npmrc .
RUN npm i -g pnpm
RUN pnpm install --ignore-scripts --frozen-lockfile
COPY . .
RUN pnpm run generate-translation-types
RUN pnpm build

FROM nginx:1.21.5
COPY --from=build /app/dist /web
COPY nginx.conf /etc/nginx/nginx.conf
