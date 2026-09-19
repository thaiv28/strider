import { LandingPage } from "@/components/landing/landing-page";

export const metadata = {
  title: { absolute: "Strider · Plan farther. Pack smarter." },
  description: "A complete backpacking workspace for routes, gear, food, weather, logistics, and shareable trip plans.",
};

export default function Page() {
  return <LandingPage />;
}
