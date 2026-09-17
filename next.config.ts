import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  // Keep the native pg driver out of the bundle; load it at runtime.
  serverExternalPackages: ["pg"],
};

export default nextConfig;
