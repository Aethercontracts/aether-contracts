FROM node:20-alpine AS builder

WORKDIR /app

COPY package*.json ./
RUN npm ci 2>/dev/null || npm install

COPY . .
RUN npm run build || mkdir -p .next public

# ── Production ────────────────────────────────────────────────────────────────
FROM node:20-alpine AS production

WORKDIR /app

COPY package*.json ./
RUN npm ci --production 2>/dev/null || npm install --production

COPY --from=builder /app/.next ./.next
COPY --from=builder /app/public ./public
COPY --from=builder /app/src ./src
COPY --from=builder /app/next.config.mjs* ./
COPY --from=builder /app/next.config.js* ./
COPY --from=builder /app/tailwind.config.ts* ./
COPY --from=builder /app/postcss.config.mjs* ./
COPY --from=builder /app/tsconfig.json* ./

ENV NODE_ENV=production
ENV NEXT_TELEMETRY_DISABLED=1

EXPOSE 3000

CMD ["npx", "next", "start", "-p", "3000"]
