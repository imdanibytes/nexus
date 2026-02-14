FROM node:20-alpine

WORKDIR /app

COPY package.json package-lock.json* ./
RUN npm install --production

COPY src/ ./src/

EXPOSE 80

CMD ["node", "src/server.js"]
