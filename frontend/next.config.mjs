/** @type {import('next').NextConfig} */
const nextConfig = {
  reactStrictMode: true,
  output: 'export',
  experimental: {
    serverActions: {
      allowedOrigins: ['https://w9.se', 'https://w9.nu']
    }
  },
  images: {
    remotePatterns: [
      {
        protocol: 'https',
        hostname: 'image.pollinations.ai'
      }
    ]
  }
}

export default nextConfig
