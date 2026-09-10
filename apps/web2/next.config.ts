import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  async rewrites() {
    const apiBaseUrl = process.env.REEL_API_URL ?? "http://localhost:3000";
    return [
      {
        source: "/v1/playback/:path*",
        destination: `${apiBaseUrl.replace(/\/$/, "")}/v1/playback/:path*`,
      },
    ];
  },
};

export default nextConfig;
