import type { Metadata, Viewport } from "next";
import "./globals.css";
import { fontVars } from "./fonts";
import { Shell } from "@/components/shell";
import { UnsavedChangesProvider } from "@/components/unsaved-changes";

const DESCRIPTION = "Backpacking trip planner and gear tracker";

export const metadata: Metadata = {
  metadataBase: new URL("https://backpack.thaiv.dev"),
  title: { default: "Strider", template: "%s · Strider" },
  description: DESCRIPTION,
  applicationName: "Strider",
  openGraph: {
    title: "Strider",
    description: DESCRIPTION,
    siteName: "Strider",
    type: "website",
    url: "/",
  },
  twitter: {
    card: "summary_large_image",
    title: "Strider",
    description: DESCRIPTION,
  },
};

export const viewport: Viewport = { width: "device-width", initialScale: 1 };

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en" className={fontVars}>
      <body>
        <UnsavedChangesProvider>
          <Shell>{children}</Shell>
        </UnsavedChangesProvider>
      </body>
    </html>
  );
}
