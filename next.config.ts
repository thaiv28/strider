import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  // Keep the native pg driver out of the bundle; load it at runtime.
  serverExternalPackages: ["pg"],
  experimental: {
    serverActions: {
      allowedOrigins: ["backpack.thaiv.dev"],
    },
  },
};

export default nextConfig;
