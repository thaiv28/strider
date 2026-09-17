import { JetBrains_Mono, Saira_Semi_Condensed, Spectral, Inter } from "next/font/google";

// Atlas type system: condensed display (map lettering), serif body, mono data,
// and a clean sans for small UI labels (eyebrows) that must stay legible.
const saira = Saira_Semi_Condensed({ subsets: ["latin"], weight: ["500", "600", "700"], variable: "--ff-saira" });
const spectral = Spectral({ subsets: ["latin"], weight: ["400", "500", "600"], variable: "--ff-spectral" });
const mono = JetBrains_Mono({ subsets: ["latin"], weight: ["400", "500", "600"], variable: "--ff-mono" });
const inter = Inter({ subsets: ["latin"], weight: ["500", "600"], variable: "--ff-inter" });

export const fontVars = [saira, spectral, mono, inter].map((f) => f.variable).join(" ");
