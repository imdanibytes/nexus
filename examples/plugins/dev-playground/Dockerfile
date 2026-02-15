FROM node:20-alpine

WORKDIR /app

COPY dist/ ./dist/
COPY server.js ./

EXPOSE 80

CMD ["node", "server.js"]
