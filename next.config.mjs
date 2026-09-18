/** @type {import("next").NextConfig} */
const nextConfig = {
  // Keep the native pg driver out of the bundle; load it at runtime.
  serverExternalPackages: ["pg"],
  experimental: {
    serverActions: {
      allowedOrigins: ["backpack.thaiv.dev"],
    },
  },
};

export default nextConfig;
