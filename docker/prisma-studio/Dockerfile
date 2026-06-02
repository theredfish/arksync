FROM node:22-alpine

WORKDIR /studio

RUN npm init -y \
    && npm install prisma

EXPOSE 5555

CMD ["sh", "-c", "npx prisma studio --url=\"$DATABASE_URL\" --port 5555 --browser none"]
