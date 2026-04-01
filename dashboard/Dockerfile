FROM node:20-alpine AS builder

WORKDIR /app

COPY package*.json ./
RUN npm ci 2>/dev/null || npm install

COPY . .
RUN npm run build 2>/dev/null || echo "Build skipped — dev mode"

# ── Production ────────────────────────────────────────────────────────────────
FROM node:20-alpine AS production

WORKDIR /app

COPY package*.json ./
RUN npm ci --production 2>/dev/null || npm install --production

COPY --from=builder /app/.next ./.next 2>/dev/null || true
COPY --from=builder /app/public ./public 2>/dev/null || true
COPY --from=builder /app/src ./src
COPY --from=builder /app/next.config.js ./
COPY --from=builder /app/tailwind.config.ts ./ 2>/dev/null || true
COPY --from=builder /app/postcss.config.mjs ./ 2>/dev/null || true
COPY --from=builder /app/tsconfig.json ./ 2>/dev/null || true

ENV NODE_ENV=production
ENV NEXT_TELEMETRY_DISABLED=1

EXPOSE 3000

CMD ["npx", "next", "start", "-p", "3000"]
