FROM node:19-alpine3.15 as build

WORKDIR /app
COPY package.json .
COPY pnpm-lock.yaml .
COPY .npmrc .
RUN npm i -g pnpm
RUN pnpm install --ignore-scripts
COPY . .
RUN pnpm build

FROM nginx:1.21.5
COPY --from=build /app/dist /web
COPY nginx.conf /etc/nginx/nginx.conf
