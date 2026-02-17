FROM node:20-alpine AS deps
WORKDIR /app
RUN echo '{"type":"module","dependencies":{"@imdanibytes/nexus-sdk":"^0.11.0"}}' > package.json
RUN npm install --omit=dev

FROM node:20-alpine
WORKDIR /app
COPY --from=deps /app/node_modules ./node_modules/
COPY --from=deps /app/package.json ./
COPY dist/ ./dist/
COPY server.js ./
EXPOSE 80
CMD ["node", "server.js"]
