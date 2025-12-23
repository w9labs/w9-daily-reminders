# Multi-stage build for w9-daily-reminders Next.js frontend
# Stage 1: Build
FROM node:20-slim AS builder

WORKDIR /build

# Copy package files
COPY package.json package-lock.json* ./

# Install dependencies
RUN npm install

# Copy source files
COPY . .

# Build the application (static export)
RUN npm run build

# Stage 2: Runtime (Nginx)
FROM nginx:alpine

# Copy built files from builder
COPY --from=builder /build/out /usr/share/nginx/html

# Copy nginx configuration
COPY nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 80

CMD ["nginx", "-g", "daemon off;"]

